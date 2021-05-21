use open_midas::reseller::Reseller;
use open_midas::calculators::AmountCalculator;
use open_midas::filters::LowAmountFilter;
use agnostic::merchant;
use agnostic_test::merchant::Merchant;
use tokio_test::block_on;

fn default_reseller<'a>(merchants: Vec<&'a dyn merchant::Merchant>) -> Reseller<'a> {
    Reseller::new(
        merchants,
        LowAmountFilter { low_amount: 0.1 },
        AmountCalculator {
            min_amount_threshold: 0.1,
            fee: 0.01,
        },
        0.1,
    )
}

#[test]
fn reseller_no_data_iteration() {
    let merchant = Merchant::default();
    let merchants: Vec<&dyn merchant::Merchant> = vec![&merchant];
    let mut reseller = default_reseller(merchants);
    let result = block_on(reseller.iterate());
    assert!(result.is_err())
}

