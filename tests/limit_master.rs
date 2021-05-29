#![feature(array_map)]
#![feature(array_methods)]
use agnostic::{
    merchant::Merchant,
    trading_pair::{Coins, Side, Target, TradingPair},
    order::OrderWithId,
    trade::Trade,
};
use agnostic_test::{
    merchant::Merchant as MerchantTest,
    trader::{TradesLogger, Trader as TraderTest}
};
use open_midas::{
    calculators::{amount_calculator::AmountCalculator, price_calculator::PriceCalculator},
    limit_master::{LimitMaster, MerchantIdManager},
};
use std::sync::Arc;

pub struct LimitMasterTestContext<const TRADERS_COUNT: usize> {
    pub traders: [Arc<TradesLogger>; TRADERS_COUNT],
    pub merchants: [Arc<dyn Merchant>; TRADERS_COUNT]
}

impl<const TRADERS_COUNT: usize> LimitMasterTestContext<TRADERS_COUNT> {
    pub fn new(
        traders: [Arc<TradesLogger>; TRADERS_COUNT],
        merchants: [Arc<dyn Merchant>; TRADERS_COUNT],
    ) -> LimitMasterTestContext<TRADERS_COUNT> {
        LimitMasterTestContext {
            traders,
            merchants,
        }
    }

    pub fn traders_with_trade(
        trades: [Trade; TRADERS_COUNT],
    ) -> Self {
        let traders = trades.map(|trade| Arc::new(TradesLogger::with_orders(
                TraderTest::default(),
                vec![trade])));
        fn create_merchant(trader: &Arc<TradesLogger>) -> Arc<dyn Merchant> {
            Arc::new(MerchantTest::with_trader(trader.clone()))
        }
        let merchants = traders.each_ref().map(create_merchant);
        LimitMasterTestContext::new(
            traders,
            merchants
        )
    }

    pub fn merchants(&self) -> [&dyn Merchant; TRADERS_COUNT] {
        self.merchants.each_ref().map(|item| item.as_ref())
    }
}

#[test]
fn default_limit_master() {
    let trading_pair = TradingPair {
        coins: Coins::TonUsdt,
        side: Side::Buy,
        target: Target::Limit,
    };
    let trade_buy = Trade::Limit(OrderWithId {
        id: "1337".into(),
        trading_pair: trading_pair.clone(),
        price: 100f64,
        amount: 100f64,
    });
    let test_context = LimitMasterTestContext::traders_with_trade(
        [trade_buy.clone(), trade_buy.clone(), trade_buy.clone(), trade_buy.clone()]);
    let merchants = test_context.merchants();
    let merchants_manager = MerchantIdManager::new(&merchants);
    let price_calculator = PriceCalculator {
        profit: 0.3f64,
    };
    let amount_calculator = AmountCalculator {
        min_amount_threshold: 1f64,
        fee: 0.01f64
    };
    let mut limit_master = LimitMaster::new(
        Coins::TonUsdt,
        merchants_manager,
        price_calculator,
        amount_calculator,
    );
    let trades = limit_master.check_current_orders();
    let trades = tokio_test::block_on(trades);
    assert!(trades.is_ok());
    let result = limit_master.update_orders();
    let result = tokio_test::block_on(result);
    assert!(result.is_ok());

    for trader in test_context.traders.iter() {
        assert_eq!(trader.create_order_log.lock().unwrap().len(), 3);
    }
}

#[test]
fn initialize_traders() {
    let trading_pair = TradingPair {
        coins: Coins::TonUsdt,
        side: Side::Sell,
        target: Target::Limit,
    };
    let trade_buy = Trade::Limit(OrderWithId {
        id: "1337".into(),
        trading_pair: trading_pair.clone(),
        price: 100f64,
        amount: 100f64,
    });
    let test_context = LimitMasterTestContext::traders_with_trade(
        [trade_buy.clone(), trade_buy.clone(), trade_buy.clone(), trade_buy.clone()]);
    let merchants = test_context.merchants();
    let merchants_manager = MerchantIdManager::new(&merchants);
    let price_calculator = PriceCalculator {
        profit: 0.3f64,
    };
    let amount_calculator = AmountCalculator {
        min_amount_threshold: 1f64,
        fee: 0.01f64
    };
    let mut limit_master = LimitMaster::new(
        Coins::TonUsdt,
        merchants_manager,
        price_calculator,
        amount_calculator,
    );
    let trades = limit_master.check_current_orders();
    let trades = tokio_test::block_on(trades);
    assert!(trades.is_ok());
    let result = limit_master.update_orders();
    let result = tokio_test::block_on(result);
    assert!(result.is_ok());

    for trader in test_context.traders.iter() {
        assert_eq!(trader.create_order_log.lock().unwrap().len(), 3);
    }
}
