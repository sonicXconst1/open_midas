#![feature(array_map)]
#![feature(array_methods)]
use agnostic::{
    merchant::Merchant,
    trading_pair::{Coins, Side, Target, TradingPair},
    order::OrderWithId,
    trade::Trade,
    market::Trader,
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

pub struct LimitMasterTestContext<'a, const TRADERS_COUNT: usize> {
    pub traders: [Arc<TradesLogger>; TRADERS_COUNT],
    pub merchants: [&'a dyn Merchant; TRADERS_COUNT]
}

impl<'a, const TRADERS_COUNT: usize> LimitMasterTestContext<'a, TRADERS_COUNT> {
    pub fn new(
        traders: [Arc<TradesLogger>; TRADERS_COUNT],
        create_merchant: impl Fn(Arc<dyn Trader>) -> &'a dyn Merchant,
    ) -> LimitMasterTestContext<'a, TRADERS_COUNT> {
        let merchants = traders.each_ref().map(|item| create_merchant(item.clone())); 
        LimitMasterTestContext {
            traders,
            merchants,
        }
    }

    pub fn with_default_traders(
        create_merchant: impl Fn(Arc<dyn Trader>) -> &'a dyn Merchant,
    ) -> Self {
        LimitMasterTestContext::new(
            [0; TRADERS_COUNT].map(|_| Arc::new(TradesLogger::default())),
            create_merchant,
        )
    }

    pub fn traders_with_trade(
        trades: [Trade; TRADERS_COUNT],
        create_merchant: impl Fn(Arc<dyn Trader>) -> &'a dyn Merchant,
    ) -> Self {
        LimitMasterTestContext::new(
            trades.map(|item| Arc::new(TradesLogger::with_orders(TraderTest::default(), vec![item]))),
            create_merchant,
        )
    }

    pub fn merchants(&self) -> &[&dyn Merchant] {
        &self.merchants
    }
}

impl<'a> LimitMasterTestContext<'a, 4> {
}

#[test]
fn default_limit_master() {
    let first_trader = Arc::new(TradesLogger::default());
    let second_trader = Arc::new(TradesLogger::default());
    let third_trader = Arc::new(TradesLogger::default());
    let fourth_trader = Arc::new(TradesLogger::default());
    let merchants: [&dyn Merchant; 4] = [
        &MerchantTest::with_trader(first_trader.clone()),
        &MerchantTest::with_trader(second_trader.clone()),
        &MerchantTest::with_trader(third_trader.clone()),
        &MerchantTest::with_trader(fourth_trader.clone()),
    ];
    let merchants_manager = MerchantIdManager::new(&merchants);
    let price_calculator = PriceCalculator { profit: 0.3f64 };
    let amount_calculator = AmountCalculator {
        min_amount_threshold: 1f64,
        fee: 0.01,
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
    println!("{:#?}", result);

    assert_eq!(first_trader.create_order_log.lock().unwrap().len(), 2);
    assert_eq!(second_trader.create_order_log.lock().unwrap().len(), 2);
    assert_eq!(third_trader.create_order_log.lock().unwrap().len(), 2);
    assert_eq!(fourth_trader.create_order_log.lock().unwrap().len(), 2);
}

#[test]
fn initialize_traders() {
    let trading_pair = TradingPair {
        coins: Coins::TonUsdt,
        side: Side::Sell,
        target: Target::Limit,
    };
    let trade_sell = Trade::Limit(OrderWithId {
        id: "1337".into(),
        trading_pair: trading_pair.clone(),
        price: 100f64,
        amount: 100f64,
    });
    let trade_buy = Trade::Limit(OrderWithId {
        id: "1337".into(),
        trading_pair: trading_pair.clone(),
        price: 100f64,
        amount: 100f64,
    });
}
