use agnostic::merchant::Merchant;
use agnostic::trade::Trade;
use agnostic::trading_pair::{Coins, Side, Target, TradingPair};
use agnostic::order::{Order, OrderWithId};
use std::collections::HashMap;

// 1. Aggregate data from merchants.
// 2. We should also check our current orders. If their volume changed - do some action: make
// a trade or something.
// 3. Update data aggregation.
// 3. Check state of currency orders. How?!

pub type MerchantId = usize;

#[derive(Clone, Debug)]
pub struct LimitMasterOrder<TOrder> {
    pub merchant_id: MerchantId,
    pub order: TOrder,
}

impl<TOrder> LimitMasterOrder<TOrder> {
    pub fn new(merchant_id: MerchantId, order: TOrder) -> Self {
        LimitMasterOrder {
            merchant_id,
            order,
        }
    }
}

#[derive(Clone, Debug)]
pub struct OrdersStorage<TOrder> {
    pub coins: Coins,
    pub stock_market: HashMap<Side, Vec<LimitMasterOrder<TOrder>>>,
    pub stock_limit: HashMap<Side, Vec<LimitMasterOrder<TOrder>>>,
}

#[allow(dead_code)]
pub struct LimitMaster<'a> {
    merchants: &'a [&'a dyn Merchant],
    my_orders_last_state: OrdersStorage<OrderWithId>,
}

impl<'a> LimitMaster<'a> {
    pub async fn iterate(&self, coins: Coins) -> Result<Trade, String> {
        let stock_orders_storage = self.accumulate_merchants_infomration(coins).await;
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

    async fn accumulate_merchants_infomration(&self, coins: Coins) -> OrdersStorage<Order> {
        self.accumulate(coins, |merchant, trading_pair| {
            let sniffer = merchant.sniffer();
            let future = async move {
                sniffer.all_the_best_orders(trading_pair, 15).await.unwrap()
            };
            Box::pin(future)
        }).await
    }

    async fn accumulate_my_current_order(&self, coins: Coins) -> OrdersStorage<OrderWithId> {
        self.accumulate(coins, |merchant, trading_pair| {
            let sniffer = merchant.sniffer();
            let future = async move {
                sniffer.get_my_orders(trading_pair).await.unwrap()
            };
            Box::pin(future)
        }).await
    }

    async fn accumulate<TOutput: std::iter::IntoIterator>(
        &self,
        coins: Coins,
        sniff_callback: impl Fn(&dyn Merchant, TradingPair) -> std::pin::Pin<Box<dyn futures::Future<Output = TOutput>>>
    ) -> OrdersStorage<TOutput::Item> {
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
                    sniff_callback(*merchant, trading_pair).await
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
