//! Limit Master
//!
//! There are two main stages: check current limit orders state & update current limit orders
//! state.
//!
//! Check: If order was updated - returns a Trade.
//!
//! Update: If there is a possibility to create better trade - do it. Or maybe it's better to
//! remove all current orders and simply place new ones?
//! Accepts price estimator. 
//! Price estimator calculate a price from orders & trading_pair.
//! Price estimator calculates the price for good order without any respect to curreny balance.
//!
//! Result of the iteration is a vector of orders.
//! Then reseller accepts trade and performs an iteration.
//!
use agnostic::merchant::Merchant;
use agnostic::trade::Trade;
use agnostic::trading_pair::{Coins, Side, Target, TradingPair};
use agnostic::order::{Order, OrderWithId};
use std::collections::HashMap;


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

pub struct MerchantIdManager<'a> {
    merchants: &'a [&'a dyn Merchant],
}

impl<'a> MerchantIdManager<'a> {
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

    pub fn iter(&self) -> std::slice::Iter<'_, &dyn Merchant> {
        self.merchants.iter()
    }
}

pub struct LimitMaster<'a> {
    merchants_manager: MerchantIdManager<'a>,
    my_orders_last_state: OrdersStorage<OrderWithId>,
}

impl<'a> LimitMaster<'a> {
    pub async fn check_current_orders(&mut self, coins: Coins) -> Result<Vec<Trade>, String> {
        let mut performed_trades = Vec::new();
        let my_current_orders = self.accumulate_my_current_order(coins).await;
        for (_last_side, last_orders) in self.my_orders_last_state.stock_limit.iter_mut() {
            for (_current_side, current_orders) in my_current_orders.stock_limit.iter() {
                for last_order in last_orders.iter_mut() {
                    for current_order in current_orders.iter() {
                        if last_order.merchant_id == current_order.merchant_id 
                            && last_order.order.id == current_order.order.id {
                            if last_order.order.amount > current_order.order.amount {
                                let mut performed_order = last_order.order.clone();
                                performed_order.amount -= current_order.order.amount;
                                last_order.order.amount = current_order.order.amount;
                                performed_trades.push(Trade::Limit(performed_order));
                            } 
                        }
                    }
                }
            }
        }
        Ok(performed_trades)
    }

    pub async fn update_orders(&mut self, coins: Coins) -> Result<(), String> {
        unimplemented!()
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
                for merchant in self.merchants_manager.iter() {
                    let trading_pair = TradingPair {
                        coins,
                        side: side.clone(),
                        target: target.clone(),
                    };
                    sniff_callback(*merchant, trading_pair).await
                        .into_iter()
                        .for_each(|order| collection.push(LimitMasterOrder::new(
                                    self.merchants_manager.get_mercahnt_id(*merchant).unwrap(),
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
