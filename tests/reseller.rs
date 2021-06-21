use agnostic::merchant;
use agnostic::order::OrderWithId;
use agnostic::trade::Trade;
use agnostic::trading_pair::{Coins, Side, Target, TradingPair};
use agnostic_test::merchant::Merchant;
use agnostic_test::sniffer::{SnifferBuilder, StockGenerator};
use open_midas::calculators::AmountCalculator;
use open_midas::filters::LowAmountFilter;
use open_midas::reseller::{Reseller, Storage};
use std::sync::Arc;
use tokio_test::block_on;

fn default_reseller<'a>(merchants: Vec<&'a dyn merchant::Merchant>) -> Reseller<'a> {
    Reseller::new(
        Storage::new(),
        Storage::new(),
        merchants,
        LowAmountFilter { low_amount: 0.1 },
        AmountCalculator {
            min_amount_threshold: 0.1,
            fee: 0.01,
        },
        0.01,
        false,
    )
}

#[test]
fn reseller_no_data_iteration() {
    let merchant = Merchant::default();
    let merchants: Vec<&dyn merchant::Merchant> = vec![&merchant];
    let mut reseller = default_reseller(merchants);
    let result = block_on(reseller.iterate());
    assert_eq!(result, Ok(None))
}

#[test]
fn reseller_with_good_orders() {
    let merchant = Merchant::with_sniffer(
        "Test",
        Arc::new(
            SnifferBuilder::new()
                .buy_stock_generator(StockGenerator::new(Side::Buy, 0.5, 0.1, 10))
                .sell_stock_generator(StockGenerator::new(Side::Sell, 1.0, 0.1, 10))
                .build(100f64),
        ),
    );
    let merchants: Vec<&dyn merchant::Merchant> = vec![&merchant];
    let mut reseller = default_reseller(merchants);
    reseller.accept_trade(Trade::Limit(OrderWithId {
        id: "1337".into(),
        trading_pair: TradingPair {
            side: Side::Buy,
            target: Target::Limit,
            coins: Coins::TonUsdt,
        },
        price: 0.49,
        amount: 10f64,
    }));
    println!("{:#?}", reseller.buy_storage);
    println!("{:#?}", reseller.sell_storage);
    let _result = block_on(reseller.iterate());
    let result = block_on(reseller.iterate());
    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(result.is_some());
    let result = result.unwrap();
    println!("{:#?}", reseller.buy_storage);
    println!("{:#?}", reseller.sell_storage);
    assert_eq!(result.price(), 0.5);
    let result = block_on(reseller.iterate());
    assert_eq!(result, Ok(None));
}
