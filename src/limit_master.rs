use agnostic::merchant::Merchant;
use agnostic::trade::Trade;
use agnostic::trading_pair::{Coins, Side, Target, TradingPair};
use agnostic::order::Order;
use std::collections::HashMap;

pub struct LimitProposal {
}

#[derive(Default, Clone, Debug)]
pub struct OrdersStorage {
    pub market: HashMap<Side, Vec<Order>>,
    pub limit: HashMap<Side, Vec<Order>>,
}

#[allow(dead_code)]
pub struct LimitMaster<'a> {
    merchants: [&'a dyn Merchant],
}

impl<'a> LimitMaster<'a> {
    pub async fn iterate(&self, coins: Coins) -> Result<Trade, String> {
        let orders_storage = self.accumulate_merchants_infomration(coins).await;
        unimplemented!()
    }

    async fn accumulate_merchants_infomration(&self, coins: Coins) -> OrdersStorage {
        let mut market_orders_collection = HashMap::new();
        let mut limit_orders_collection = HashMap::new();
        for side in &[Side::Sell, Side::Buy] {
            for target in &[Target::Market, Target::Limit] {
                let mut collection = Vec::new();
                for merchant in self.merchants.iter() {
                    let trading_pair = TradingPair {
                        coins,
                        side: side.clone(),
                        target: target.clone(),
                    };
                    let sniffer = merchant.sniffer();
                    sniffer.all_the_best_orders(trading_pair, 15).await
                        .unwrap()
                        .into_iter()
                        .for_each(|order| collection.push(order));
                };
                match target {
                    Target::Market => market_orders_collection.insert(*side, collection).unwrap(),
                    Target::Limit => limit_orders_collection.insert(*side, collection).unwrap(),
                };
            };
        };
        OrdersStorage {
            market: market_orders_collection,
            limit: limit_orders_collection,
        }
    }
}
