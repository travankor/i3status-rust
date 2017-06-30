use std::time::Duration;
use std::sync::mpsc::Sender;

use block::{Block, ConfigBlock};
use config::Config;
use de::deserialize_duration;
use errors::*;
use widgets::text::TextWidget;
use widget::I3BarWidget;
use input::I3BarEvent;
use scheduler::Task;
use std::fs::OpenOptions;
use std::io::prelude::*;

use uuid::Uuid;

pub struct Net {
    output: TextWidget,
    id: String,
    update_interval: Duration,
    device_path: String,
    rx_bytes: u64,
    tx_bytes: u64,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct NetConfig {
    /// Update interval in seconds
    #[serde(default = "NetConfig::default_interval", deserialize_with = "deserialize_duration")]
    pub interval: Duration,

    /// Which interface in /sys/class/net/ to read from.
    //#[serde(default = "NetConfig::default_device")]
    pub device: String,
}

impl NetConfig {
    fn default_interval() -> Duration {
        Duration::from_secs(1)
    }
}

impl ConfigBlock for Net {
    type Config = NetConfig;

    fn new(block_config: Self::Config, config: Config, tx_update_request: Sender<Task>) -> Result<Self> {
        Ok(Net {
            id: Uuid::new_v4().simple().to_string(),
            update_interval: block_config.interval,
            output: TextWidget::new(config.clone()).with_text("Net"),
            device_path: format!("/sys/class/net/{}/statistics/", block_config.device),
            rx_bytes: 0,
            tx_bytes: 0,
        })
    }
}

fn read_file(path: &str) -> Result<String> {
    let mut f = OpenOptions::new().read(true).open(path).block_error(
        "net",
        &format!("failed to open file {}", path),
    )?;
    let mut content = String::new();
    f.read_to_string(&mut content).block_error(
        "net",
        &format!("failed to read {}", path),
    )?;
    // Removes trailing newline
    content.pop();
    Ok(content)
}

fn convert_speed(speed: u64) -> (f64, &'static str) {
    // the values for the match are so the speed doesn't go above 3 characters
    let (speed, unit) = match speed {
        x if x > 1047527424 => {(speed as f64 / 1073741824.0, "G")},
        x if x > 1022976 => {(speed as f64 / 1048576.0, "M")},
        x if x > 999 => {(speed as f64 / 1024.0, "K")},
        _ => (speed as f64, "B"),
    };
    (speed, unit)
}

fn make_graph(values: &Vec<u64>) -> &'static str{
    let bars = ["_","▁","▂","▃","▄","▅","▆","▇","█"];
    let min = values.iter().min().unwrap();
    let max = values.iter().max().unwrap();
    let extant = (max - min) as usize;
    let bar = values.into_iter()
                    .map(|x| bars[(x - min) as usize / (extant * (bars.len() - 1))])
                    .collect::<Vec<&'static str>>()
                    .concat();
    bars[0]
}
impl Block for Net {
    fn update(&mut self) -> Result<Option<Duration>> {
        let current_rx = read_file(&format!("{}rx_bytes", self.device_path))?
            .parse::<u64>()
            .block_error("net", "failed to parse rx_bytes")?;
        let (rx_speed, rx_unit) = convert_speed((current_rx - self.rx_bytes) / self.update_interval.as_secs());
        self.rx_bytes = current_rx;

        let current_tx = read_file(&format!("{}tx_bytes", self.device_path))?
            .parse::<u64>()
            .block_error("net", "failed to parse tx_bytes")?;
        let (tx_speed, tx_unit) = convert_speed((current_tx - self.tx_bytes) / self.update_interval.as_secs());
        self.tx_bytes = current_tx;

        self.output.set_text(format!("⬆ {:6.1}{} ⬇ {:6.1}{}", tx_speed, tx_unit, rx_speed, rx_unit));
        Ok(Some(self.update_interval))
    }

    fn view(&self) -> Vec<&I3BarWidget> {
        vec![&self.output]
    }

    fn click(&mut self, _: &I3BarEvent) -> Result<()> {
        Ok(())
    }

    fn id(&self) -> &str {
        &self.id
    }
}
