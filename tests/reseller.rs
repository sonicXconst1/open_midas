use open_midas::reseller::{Reseller, Storage};
use open_midas::calculators::AmountCalculator;
use open_midas::filters::LowAmountFilter;
use agnostic::merchant;
use agnostic_test::merchant::Merchant;
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
        0.1,
        true,
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
}
