use crate::book::Book;
use crate::observations::Observation;
use crate::recovery::Recovery;
use crate::username::Username;
use reqwest::Error;

macro_rules! url {
    ($auto_trader:expr, $port:expr, $endpoint:expr) => {
        format!("http://{}:{}/{}", $auto_trader.host, $port, $endpoint)
    };
}

trait ConstantPorts {
    const OBSERVATION_PORT: u16;
    const EXECUTION_PORT: u16;
    const FEED_RECOVERY_PORT: u16;
}

pub struct AutoTrader {
    pub username: Username,
    pub password: String,
    pub host: String,
}

impl ConstantPorts for AutoTrader {
    const OBSERVATION_PORT: u16 = 8090;

    const EXECUTION_PORT: u16 = 9050;

    const FEED_RECOVERY_PORT: u16 = 9000;
}

impl AutoTrader {
    pub async fn startup(&self) -> Result<(), Error> {
        self.recover().await
    }

    async fn recover(&self) -> Result<(), Error> {
        let response: Vec<Recovery> =
            reqwest::get(url!(self, AutoTrader::FEED_RECOVERY_PORT, "recover"))
                .await?
                .json()
                .await?;
        for message in response {
            match message {
                Recovery::Future(_) => todo!(),
                Recovery::Trade(_) => todo!(),
                Recovery::Added(_) => todo!(),
                Recovery::Index(_) => todo!(),
            }
        }
        Ok(())
    }

    pub async fn refresh_latest_observations(&self) -> Result<(), Error> {
        let response: Vec<Observation> =
            reqwest::get(url!(self, AutoTrader::OBSERVATION_PORT, "current"))
                .await?
                .json()
                .await?;
        println!("{response:#?}");
        Ok(())
    }
}
