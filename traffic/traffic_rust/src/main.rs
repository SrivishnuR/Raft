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

enum Lights {
    Ns,
    Ew,
}

struct TrafficController {
    state: States,
    ticks: u8,
    button_pressed: bool,
    socket: UdpSocket,
}

impl TrafficController {
    pub async fn new() -> TrafficController {
        let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 10001)).await.unwrap();

        let controller = TrafficController {
            state: States::NsGreen,
            ticks: 0,
            button_pressed: false,
            socket,
        };

        controller.send_to(Lights::Ns, 'G').await;
        controller.send_to(Lights::Ew, 'R').await;
        controller
    }

    async fn send_to(self: &Self, light: Lights, color: char) {
        let mut color_buffer = [0; 1];
        color.encode_utf8(&mut color_buffer);

        match light {
            Lights::Ew => {
                self.socket
                    .send_to(&color_buffer, (Ipv4Addr::LOCALHOST, 11000))
                    .await
                    .unwrap();
            }
            Lights::Ns => {
                self.socket
                    .send_to(&color_buffer, (Ipv4Addr::LOCALHOST, 12000))
                    .await
                    .unwrap();
            }
        }
    }

    pub async fn process_clock_tick(self: &mut Self) {
        self.ticks += 1;

        match self.state {
            States::NsGreen => {
                if self.ticks >= 60 || (self.ticks >= 15 && self.button_pressed) {
                    self.state = States::NsYellow;
                    self.ticks = 0;
                    self.button_pressed = false;
                    self.send_to(Lights::Ns, 'Y').await;
                }
            }
            States::NsYellow => {
                if self.ticks >= 5 {
                    self.state = States::EwGreen;
                    self.ticks = 0;
                    self.send_to(Lights::Ns, 'R').await;
                    self.send_to(Lights::Ew, 'G').await;
                }
            }
            States::EwGreen => {
                if self.ticks >= 30 || (self.ticks >= 15 && self.button_pressed) {
                    self.state = States::EwYellow;
                    self.ticks = 0;
                    self.button_pressed = false;
                    self.send_to(Lights::Ew, 'Y').await;
                }
            }
            States::EwYellow => {
                if self.ticks >= 5 {
                    self.state = States::EwGreen;
                    self.ticks = 0;
                    self.send_to(Lights::Ew, 'R').await;
                    self.send_to(Lights::Ns, 'G').await;
                }
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

    let mut controller = TrafficController::new().await;
    loop {
        let command = rx.recv().await;

        if let Some(command) = command {
            match command {
                Commands::ClockTick => controller.process_clock_tick().await,
                Commands::PressEw => controller.process_ew_press(),
                Commands::PressNs => controller.process_ns_press(),
            }
        }
    }
}

async fn clock(tx: mpsc::Sender<Commands>) {
    loop {
        sleep(Duration::from_secs(1)).await;
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
