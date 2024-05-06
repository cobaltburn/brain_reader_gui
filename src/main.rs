use brainflow::brainflow_input_params::BrainFlowInputParamsBuilder;
use brainflow::{board_shim, BoardIds, BrainFlowPresets};
use iced::widget::{
    button, column, container, container::Appearance, row, space::Space, text, Container,
};
use iced::{
    alignment::{Horizontal, Vertical},
    border::Radius,
    Background, Border, Color, Element, Length, Sandbox, Settings, Shadow,
};
use once_cell::sync::Lazy;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tello::CommandMode;
use tokio::runtime::Runtime;

static DRONE: Lazy<Arc<Mutex<CommandMode>>> = Lazy::new(|| {
    let rt = Runtime::new().unwrap();
    let drone = rt.block_on(CommandMode::new("192.168.10.1:8889")).unwrap();
    Arc::new(Mutex::new(drone))
});

#[derive(Debug, Deserialize, Serialize)]
struct Prediction {
    prediction_label: String,
    prediction_count: usize,
}

#[derive(Debug, Copy, Clone)]
enum Movements {
    Takeoff,
    Right,
    Left,
    Land,
    Forward,
    Backward,
    None,
}

impl From<&str> for Movements {
    fn from(value: &str) -> Self {
        match value {
            "takeoff" => Movements::Takeoff,
            "right" => Movements::Right,
            "left" => Movements::Left,
            "land" => Movements::Land,
            "forward" => Movements::Forward,
            "backward" => Movements::Backward,
            "Takeoff" => Movements::Takeoff,
            "Right" => Movements::Right,
            "Left" => Movements::Left,
            "Land" => Movements::Land,
            "Forward" => Movements::Forward,
            "Backward" => Movements::Backward,
            _ => Movements::None,
        }
    }
}

fn main() -> Result<(), iced::Error> {
    PredictionWindow::run(Settings::default())
}

#[derive(Debug, Default)]
struct PredictionWindow {
    movement: String,
    reading_counter: usize,
    history: Vec<Prediction>,
    connection: bool,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    ReadBrain,
    Connect,
    Execute,
    Takeoff,
    Land,
}

impl Sandbox for PredictionWindow {
    type Message = Message;

    fn new() -> Self {
        PredictionWindow {
            movement: String::new(),
            reading_counter: 0,
            history: Vec::new(),
            connection: false,
        }
    }

    fn title(&self) -> String {
        String::from("Brain Reader page")
    }
    fn update(&mut self, message: Self::Message) {
        match message {
            Message::ReadBrain => {
                let readings = read_cyton_board();
                let Ok(readings) = readings else {
                    eprintln!("{:?}", readings);
                    return;
                };
                let Ok(rt) = Runtime::new() else {
                    eprintln!("could not generate run time");
                    return;
                };

                let url = "http://127.0.0.1:5000/prediction";
                let json = rt
                    .block_on(Client::post(&Client::new(), url).json(&readings).send())
                    .and_then(|response| rt.block_on(response.json::<Prediction>()));

                let Ok(json) = json else {
                    eprintln!("{:?}", json);
                    return;
                };

                self.movement = json.prediction_label.clone();
                self.reading_counter = json.prediction_count;
                self.history.push(json);
            }
            Message::Connect => {
                if self.connection {
                    return;
                }
                let Ok(drone) = DRONE.try_lock() else {
                    eprintln!("Unable to obtain a lock on the drone");
                    return;
                };
                let Ok(rt) = Runtime::new() else {
                    eprintln!("unable to bind runtime");
                    return;
                };
                let res = rt.block_on(drone.enable());
                let Ok(_) = res else {
                    eprintln!("{:?}", res);
                    return;
                };
                self.connection = true;
            }
            Message::Execute => {
                if !self.connection {
                    return;
                }
                let movement = self.movement.clone();

                let Ok(mut drone) = DRONE.try_lock() else {
                    eprintln!("Unable to obtain a lock on the drone");
                    return;
                };

                let Ok(rt) = Runtime::new() else {
                    eprintln!("unable to bind runtime");
                    return;
                };
                let _ = match Movements::from(movement.as_str()) {
                    Movements::Takeoff => rt.block_on(drone.take_off()),
                    Movements::Land => rt.block_on(drone.land()),
                    Movements::Right => rt.block_on(drone.cw(90)),
                    Movements::Left => rt.block_on(drone.ccw(90)),
                    Movements::Forward => rt.block_on(drone.forward(100)),
                    Movements::Backward => rt.block_on(drone.back(100)),
                    Movements::None => Ok(()),
                };

                // fire and forget method may or may not work didn't feel like finding out
                /* let Ok(rt) = Runtime::new() else {
                    eprintln!("unable to bind runtime");
                    return;
                };
                rt.spawn(async move {
                    let Ok(mut drone) = DRONE.try_lock() else {
                        eprintln!("Unable to obtain a lock on the drone");
                        return;
                    };

                    let Ok(rt) = Runtime::new() else {
                        eprintln!("unable to bind runtime");
                        return;
                    };
                    let _ = match Movements::from(movement.as_str()) {
                        Movements::Takeoff => rt.block_on(drone.take_off()),
                        Movements::Land => rt.block_on(drone.land()),
                        Movements::Right => rt.block_on(drone.cw(90)),
                        Movements::Left => rt.block_on(drone.ccw(90)),
                        Movements::Forward => rt.block_on(drone.forward(100)),
                        Movements::Backward => rt.block_on(drone.back(100)),
                        Movements::None => Ok(()),
                    };
                }); */
            }
            Message::Takeoff => {
                if !self.connection {
                    return;
                }
                let Ok(mut drone) = DRONE.try_lock() else {
                    eprintln!("Unable to obtain a lock on the drone");
                    return;
                };

                let Ok(rt) = Runtime::new() else {
                    eprintln!("unable to bind runtime");
                    return;
                };
                let _ = rt.block_on(drone.take_off());
            }
            Message::Land => {
                if !self.connection {
                    return;
                }
                let Ok(drone) = DRONE.try_lock() else {
                    eprintln!("Unable to obtain a lock on the drone");
                    return;
                };

                let Ok(rt) = Runtime::new() else {
                    eprintln!("unable to bind runtime");
                    return;
                };
                let _ = rt.block_on(drone.land());
            }
        }
    }

    fn view(&self) -> iced::Element<Self::Message> {
        let read_brain_waves = button(
            text("Read my mind")
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center),
        )
        .on_press(Message::ReadBrain)
        .width(200)
        .height(75);
        let execute = button(
            text("Excute Reading")
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center),
        )
        .on_press(Message::Execute)
        .width(200)
        .height(50);
        let movement_prediction = apply_black_boarder(
            text(self.movement.clone())
                .width(Length::Fill)
                .horizontal_alignment(Horizontal::Center),
        )
        .width(100);
        let counter = apply_black_boarder(
            text(self.reading_counter)
                .width(Length::Fill)
                .horizontal_alignment(Horizontal::Center),
        )
        .width(100);
        let count_label = apply_black_boarder(
            text("count")
                .width(Length::Fill)
                .horizontal_alignment(Horizontal::Center),
        )
        .width(100);
        let movement_label = apply_black_boarder(
            text("movement")
                .width(Length::Fill)
                .horizontal_alignment(Horizontal::Center),
        )
        .width(100);

        let reading_window = container(
            column![
                Space::with_height(50),
                read_brain_waves,
                column![
                    row![count_label, movement_label],
                    row![counter, movement_prediction]
                ],
                execute,
            ]
            .spacing(10),
        )
        .height(Length::FillPortion(1));

        let count_history_label = text("Predictions Count")
            .horizontal_alignment(Horizontal::Center)
            .width(Length::FillPortion(1));
        let prediciton_history_label = text("Server Predictions")
            .horizontal_alignment(Horizontal::Center)
            .width(Length::FillPortion(1));
        let count_history = text(display_count(&self.history))
            .horizontal_alignment(Horizontal::Center)
            .width(Length::FillPortion(1));
        let prediciton_history = text(display_prediction(&self.history))
            .horizontal_alignment(Horizontal::Center)
            .width(Length::FillPortion(1));

        let prediction_history_window = column![
            Space::with_height(50),
            apply_black_boarder(column![
                apply_black_boarder(row![count_history_label, prediciton_history_label]),
                row![count_history, prediciton_history]
            ])
            .width(Length::FillPortion(1))
            .height(Length::FillPortion(2)),
            Space::with_height(Length::FillPortion(2)),
        ];

        let connect = button(
            text("Connect")
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center),
        )
        .on_press(Message::Connect)
        .width(100)
        .height(100);

        let takeoff = button(
            text("takeoff")
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center),
        )
        .on_press(Message::Takeoff)
        .width(50)
        .height(50);
        let land = button(
            text("land")
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center),
        )
        .on_press(Message::Land)
        .width(50)
        .height(50);

        let view = row![
            Space::with_width(Length::FillPortion(1)),
            column![
                reading_window,
                connect,
                Space::with_width(100),
                row![takeoff, land],
                Space::with_height(Length::FillPortion(1))
            ]
            .width(Length::FillPortion(1)),
            Space::with_width(50),
            prediction_history_window,
            Space::with_width(Length::FillPortion(1))
        ];

        Container::new(view)
            .center_x()
            .center_y()
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn theme(&self) -> iced::Theme {
        iced::Theme::Dark
    }
}

fn apply_black_boarder<'a>(
    content: impl Into<Element<'a, Message, iced::Theme, iced::Renderer>>,
) -> Container<'a, Message> {
    let border = Border {
        color: Color::BLACK,
        width: 1.0,
        radius: Radius::default(),
    };
    container(content).style(Appearance {
        text_color: Some(Color::BLACK),
        background: Some(Background::Color(Color::WHITE)),
        border,
        shadow: Shadow::default(),
    })
}

fn display_prediction(history: &Vec<Prediction>) -> String {
    let mut predictions = String::new();
    history
        .iter()
        .for_each(|e| predictions = format!("{}\n{}", e.prediction_label, predictions));
    predictions
}

fn display_count(history: &Vec<Prediction>) -> String {
    let mut counts = String::new();
    history
        .iter()
        .for_each(|e| counts = format!("{}\n{}", e.prediction_count, counts));
    counts
}

fn read_cyton_board() -> anyhow::Result<HashMap<String, Vec<f64>>> {
    let params = BrainFlowInputParamsBuilder::default()
        .serial_port("/dev/ttyUSB0")
        .build();
    let board = board_shim::BoardShim::new(BoardIds::CytonDaisyBoard, params)?;
    board.prepare_session()?;

    board.start_stream(45000, "")?;
    thread::sleep(Duration::from_secs(10));

    board.stop_stream()?;
    let data = board.get_board_data(None, BrainFlowPresets::DefaultPreset)?;
    board.release_session()?;
    println!("{:?}", data.view());

    let mut readings = HashMap::new();

    // rows from the board represent columns in the tensor
    for (i, arr) in data.rows().into_iter().enumerate() {
        let mut column = vec![];
        for j in arr.into_owned() {
            column.push(j);
        }
        readings.insert(format!("c{}", i), column);
    }

    Ok(readings)
}

#[allow(dead_code)]
fn read_synthetic_board() -> anyhow::Result<HashMap<String, Vec<f64>>> {
    brainflow::board_shim::enable_dev_board_logger()?;
    let params = BrainFlowInputParamsBuilder::default().build();
    let board = board_shim::BoardShim::new(BoardIds::SyntheticBoard, params)?;

    board.prepare_session()?;
    board.start_stream(45000, "")?;
    thread::sleep(Duration::from_secs(5));
    board.stop_stream()?;
    let data = board.get_board_data(None, BrainFlowPresets::DefaultPreset)?;
    board.release_session()?;

    let mut readings = HashMap::new();

    // rows from the board represent columns in the tensor
    for (i, arr) in data.rows().into_iter().enumerate() {
        let mut column = vec![];
        for j in arr.into_owned() {
            column.push(j);
        }
        readings.insert(format!("c{}", i), column);
    }

    Ok(readings)
}

#[allow(dead_code)]
fn csv_to_json(path: &str) -> anyhow::Result<HashMap<String, Vec<f64>>> {
    let mut dir = std::env::current_dir()?;
    dir.push(format!("{}", path));

    let mut records = csv::Reader::from_path(dir)?.into_records();
    let mut readings: HashMap<String, Vec<f64>> = HashMap::new();

    while let Some(Ok(record)) = records.next() {
        for (i, val) in record.iter().enumerate() {
            if i == 32 {
                break;
            }
            let column = format!("c{}", i);
            let val: f64 = val.parse()?;
            readings
                .entry(column)
                .and_modify(|e| e.push(val))
                .or_insert(vec![val]);
        }
    }
    Ok(readings)
}
