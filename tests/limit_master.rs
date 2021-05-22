use agnostic::{merchant::Merchant, trading_pair::Coins};
use agnostic_test::{merchant::Merchant as MerchantTest, trader::TradesLogger};
use open_midas::{
    calculators::{amount_calculator::AmountCalculator, price_calculator::PriceCalculator},
    limit_master::{LimitMaster, MerchantIdManager},
};
use std::sync::Arc;

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
