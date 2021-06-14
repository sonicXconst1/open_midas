use crate::bookkeeper::Coins;
use crate::reseller::{Entry, Reseller, Storage};
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use agnostic::trading_pair;

pub struct ResellerSaver {
    file: std::fs::File,
}

impl ResellerSaver {
    pub fn save_storages(&mut self, reseller: &Reseller) -> Result<(), std::io::Error> {
        let storages = [
            convert_storage(&reseller.buy_storage),
            convert_storage(&reseller.sell_storage),
        ];
        self.file.seek(SeekFrom::Start(0))?;
        self.file.set_len(0)?;
        self.file.write_all(&serde_json::to_vec(&storages)?)
    }

    pub fn load(file: impl Into<PathBuf>) -> Result<ResellerSaver, std::io::Error> {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open(&file.into())?;
        Ok(ResellerSaver { file })
    }

    pub fn read_buy_and_sell_storages(
        &mut self,
    ) -> Result<(Storage, Storage), std::io::Error> {
        self.file.seek(SeekFrom::Start(0))?;
        let mut storages = String::with_capacity(100);
        self.file.read_to_string(&mut storages)?;
        let storages: [HashMap<Coins, Vec<Entry>>; 2] = serde_json::from_str(&storages)?;
        Ok((
            to_storage(storages.get(0).unwrap()),
            to_storage(storages.get(1).unwrap()),
        ))
    }
}

fn convert_storage(storage: &Storage) -> HashMap<Coins, &Vec<Entry>> {
    storage
        .iter()
        .map(|(key, value)| (key.clone().into(), value))
        .collect()
}

fn to_storage(storage: &HashMap<Coins, Vec<Entry>>) -> Storage {
    let mut map: Storage = HashMap::new();
    storage
        .iter()
        .for_each(|(key, value)| {
            map.insert(
                trading_pair::Coins::from(key.clone()),
                value.clone());
        });
    map
}
