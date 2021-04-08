use agnostic::merchant::Merchant;
use agnostic::trade::Trade;
use agnostic::trading_pair::{Coins, Side, Target, TradingPair};
use agnostic::order::Order;

pub struct LimitProposal {
}

#[derive(Default, Clone, Debug)]
pub struct OrdersStorage {
    pub market: Vec<Order>,
    pub limit: Vec<Order>,
}

#[allow(dead_code)]
pub struct LimitMaster<'a> {
    merchants: [&'a dyn Merchant],
}

impl<'a> LimitMaster<'a> {
    pub async fn iterate(&self, coins: Coins) -> Result<Trade, String> {
        let _orders_storage = self.accumulate_merchants_infomration(coins).await;
        unimplemented!()
    }

    async fn accumulate_merchants_infomration(&self, coins: Coins) -> OrdersStorage {
        let mut market_orders_collection = Vec::new();
        let mut limit_orders_collection = Vec::new();
        for side in &[Side::Sell, Side::Buy] {
            for target in &[Target::Market, Target::Limit] {
                let collection = match target {
                    Target::Market => &mut market_orders_collection,
                    Target::Limit => &mut limit_orders_collection,
                };
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
            };
        };
        OrdersStorage {
            market: market_orders_collection,
            limit: limit_orders_collection,
        }
    }
}
