use std::str;
use std::{error::Error, net::Ipv4Addr};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

enum Commands {
    PressNs,
    PressEw,
    ClockTick,
}

// Possible traffic light states
#[derive(PartialEq)]
enum States {
    NsGreen,
    NsYellow,
    EwGreen,
    EwYellow,
}

#[derive(PartialEq, Debug)]
enum Lights {
    Ns,
    Ew,
}

struct TrafficController {
    state: States,
    ticks: u8,
    button_pressed: bool,
}

impl TrafficController {
    pub fn new(initial_state: States) -> TrafficController {
        let controller = TrafficController {
            state: initial_state,
            ticks: 0,
            button_pressed: false,
        };

        controller
    }

    pub fn process_clock_tick(self: &mut Self) -> Option<Vec<(Lights, char)>> {
        self.ticks += 1;

        match self.state {
            States::NsGreen => {
                if self.ticks >= 60 || (self.ticks >= 15 && self.button_pressed) {
                    self.state = States::NsYellow;
                    self.ticks = 0;
                    self.button_pressed = false;
                    return Some(vec![(Lights::Ns, 'Y')]);
                }

                None
            }
            States::NsYellow => {
                if self.ticks >= 5 {
                    self.state = States::EwGreen;
                    self.ticks = 0;
                    return Some(vec![(Lights::Ns, 'R'), (Lights::Ew, 'G')]);
                }

                None
            }
            States::EwGreen => {
                if self.ticks >= 30 || (self.ticks >= 15 && self.button_pressed) {
                    self.state = States::EwYellow;
                    self.ticks = 0;
                    self.button_pressed = false;
                    return Some(vec![(Lights::Ew, 'Y')]);
                }

                None
            }
            States::EwYellow => {
                if self.ticks >= 5 {
                    self.state = States::NsGreen;
                    self.ticks = 0;
                    return Some(vec![(Lights::Ew, 'R'), (Lights::Ns, 'G')]);
                }

                None
            }
        }
    }

    pub fn process_ew_press(self: &mut Self) {
        if self.state == States::EwGreen {
            self.button_pressed = true;
        }
    }

    pub fn process_ns_press(self: &mut Self) {
        if self.state == States::NsGreen {
            self.button_pressed = true;
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let (tx, mut rx) = mpsc::channel::<Commands>(100);
    tokio::spawn(button_listener(tx.clone()));
    tokio::spawn(clock(tx.clone()));

    // Init traffic lights
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 10001)).await.unwrap();
    send_to(&socket, Lights::Ns, 'G').await;
    send_to(&socket, Lights::Ew, 'R').await;

    // Init controller
    let mut controller = TrafficController::new(States::NsGreen);

    loop {
        let command = rx.recv().await;

        if let Some(command) = command {
            match command {
                Commands::ClockTick => {
                    let changes = controller.process_clock_tick();
                    if let Some(changes) = changes {
                        for change in changes {
                            send_to(&socket, change.0, change.1).await;
                        }
                    }
                }
                Commands::PressEw => controller.process_ew_press(),
                Commands::PressNs => controller.process_ns_press(),
            }
        }
    }
}

async fn clock(tx: mpsc::Sender<Commands>) {
    loop {
        sleep(Duration::from_millis(1000)).await;
        tx.send(Commands::ClockTick).await.unwrap();
    }
}

async fn button_listener(tx: mpsc::Sender<Commands>) {
    let sock = UdpSocket::bind((Ipv4Addr::LOCALHOST, 10000)).await.unwrap();

    let mut buf = [0; 8];
    loop {
        sock.recv_from(&mut buf).await.unwrap();
        let command = str::from_utf8(&buf)
            .unwrap()
            .trim()
            .trim_matches(char::from(0));

        match command {
            "NS" => tx.send(Commands::PressNs).await.unwrap(),
            "EW" => tx.send(Commands::PressEw).await.unwrap(),
            _ => println!("{}", command),
        }
    }
}

async fn send_to(socket: &UdpSocket, light: Lights, color: char) {
    let mut color_buffer = [0; 1];
    color.encode_utf8(&mut color_buffer);

    match light {
        Lights::Ew => {
            socket
                .send_to(&color_buffer, (Ipv4Addr::LOCALHOST, 11000))
                .await
                .unwrap();
        }
        Lights::Ns => {
            socket
                .send_to(&color_buffer, (Ipv4Addr::LOCALHOST, 12000))
                .await
                .unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transition_1() {
        let mut controller = TrafficController::new(States::NsGreen);

        for _ in 0..59 {
            controller.process_clock_tick();
        }

        let state_change = controller.process_clock_tick();
        assert_eq!(state_change, Some(vec![(Lights::Ns, 'Y')]));
    }

    #[test]
    fn test_transition_2() {
        let mut controller = TrafficController::new(States::NsGreen);

        for _ in 0..64 {
            controller.process_clock_tick();
        }

        let state_change = controller.process_clock_tick();
        assert_eq!(
            state_change,
            Some(vec![(Lights::Ns, 'R'), (Lights::Ew, 'G')])
        );
    }

    #[test]
    fn test_transition_3() {
        let mut controller = TrafficController::new(States::NsGreen);

        for _ in 0..94 {
            controller.process_clock_tick();
        }

        let state_change = controller.process_clock_tick();
        assert_eq!(state_change, Some(vec![(Lights::Ew, 'Y')]));
    }

    #[test]
    fn test_transition_4() {
        let mut controller = TrafficController::new(States::NsGreen);

        for _ in 0..99 {
            controller.process_clock_tick();
        }

        let state_change = controller.process_clock_tick();
        assert_eq!(
            state_change,
            Some(vec![(Lights::Ew, 'R'), (Lights::Ns, 'G')])
        );
    }

    #[test]
    fn test_button_before_15() {
        let mut controller = TrafficController::new(States::NsGreen);

        controller.process_ns_press();
        for _ in 0..14 {
            assert_eq!(None, controller.process_clock_tick());
        }

        assert_eq!(
            Some(vec![(Lights::Ns, 'Y')]),
            controller.process_clock_tick()
        );
    }

    #[test]
    fn test_button_after_15() {
        let mut controller = TrafficController::new(States::NsGreen);

        for _ in 0..15 {
            assert_eq!(None, controller.process_clock_tick());
        }

        controller.process_ns_press();

        assert_eq!(
            Some(vec![(Lights::Ns, 'Y')]),
            controller.process_clock_tick()
        );
    }
}
