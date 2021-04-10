use agnostic::merchant::Merchant;
use agnostic::trade::Trade;
use agnostic::trading_pair::{Coins, Side, Target, TradingPair};
use agnostic::order::Order;
use std::collections::HashMap;

// 1. Aggregate data from merchants.
// 2. Check state of currency orders. How?!

pub type MerchantId = usize;

pub struct LimitProposal {
}

#[derive(Clone, Debug)]
pub struct LimitMasterOrder {
    pub merchant_id: MerchantId,
    pub order: Order,
}

impl LimitMasterOrder {
    pub fn new(merchant_id: MerchantId, order: Order) -> Self {
        LimitMasterOrder {
            merchant_id,
            order,
        }
    }
}

#[derive(Clone, Debug)]
pub struct OrdersStorage {
    pub coins: Coins,
    pub stock_market: HashMap<Side, Vec<LimitMasterOrder>>,
    pub stock_limit: HashMap<Side, Vec<LimitMasterOrder>>,
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

    pub fn get_mercahnt_id(&self, merchant: &dyn Merchant) -> Option<MerchantId> {
        self.merchants
            .iter()
            .enumerate()
            .find(|(_index, m)| std::ptr::eq(**m, merchant))
            .map(|(index, _merchant)| index)
    }

    pub fn get_merchant(&self, id: MerchantId) -> Option<&dyn Merchant> {
        self.merchants.get(id).map(|value| *value)
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
                        .for_each(|order| collection.push(LimitMasterOrder::new(
                                    self.get_mercahnt_id(*merchant).unwrap(),
                                    order)));
                };
                match target {
                    Target::Market => market_orders_collection.insert(*side, collection).unwrap(),
                    Target::Limit => limit_orders_collection.insert(*side, collection).unwrap(),
                };
            };
        };
        OrdersStorage {
            coins,
            stock_market: market_orders_collection,
            stock_limit: limit_orders_collection,
        }
    }
}
