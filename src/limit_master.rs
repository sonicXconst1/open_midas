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

pub type MerchantId = usize;

#[derive(Clone, Debug)]
pub struct OrderEntity<TOrder> {
    pub merchant_id: MerchantId,
    pub order: TOrder,
}

impl<TOrder> OrderEntity<TOrder> {
    pub fn new(merchant_id: MerchantId, order: TOrder) -> Self {
        OrderEntity {
            merchant_id,
            order,
        }
    }
}

#[derive(Clone, Debug)]
pub struct OrdersStorage<TOrder> {
    pub coins: Coins,
    pub sell_stock: Vec<OrderEntity<TOrder>>,
    pub buy_stock: Vec<OrderEntity<TOrder>>,
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
        for last_order in self.my_orders_last_state.buy_stock.iter_mut() {
            for current_order in my_current_orders.buy_stock.iter() {
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
        Ok(performed_trades)
    }

    pub async fn update_orders(&mut self, coins: Coins) -> Result<(), String> {
        let _orders_storage = self.accumulate_merchants_infomration(coins).await;
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
        let mut sell_orders_collection = Vec::new();
        let mut buy_orders_collection = Vec::new();
        for merchant in self.merchants_manager.iter() {
            let trading_pair = TradingPair {
                coins,
                side: Side::Sell,
                target: Target::Limit,
            };
            sniff_callback(*merchant, trading_pair).await
                .into_iter()
                .for_each(|order| sell_orders_collection.push(OrderEntity::new(
                            self.merchants_manager.get_mercahnt_id(*merchant).unwrap(),
                            order)));
        };
        for merchant in self.merchants_manager.iter() {
            let trading_pair = TradingPair {
                coins,
                side: Side::Buy,
                target: Target::Limit,
            };
            sniff_callback(*merchant, trading_pair).await
                .into_iter()
                .for_each(|order| buy_orders_collection.push(OrderEntity::new(
                            self.merchants_manager.get_mercahnt_id(*merchant).unwrap(),
                            order)));
        };
        OrdersStorage {
            coins,
            buy_stock: buy_orders_collection,
            sell_stock: sell_orders_collection,
        }
    }
}
