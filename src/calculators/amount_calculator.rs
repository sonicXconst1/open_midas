use agnostic::order::Order;

#[derive(Debug)]
pub struct AmountCalculator {
    pub min_amount_threshold: f64,
    pub fee: f64,
}

#[derive(PartialEq, Debug)]
pub enum Amount {
    PriceBased(f64),
    BalanceBased(f64),
}

impl Amount {
    pub fn value(&self) -> f64 {
        match self {
            Amount::PriceBased(value) => *value,
            Amount::BalanceBased(value) => *value,
        }
    }
}

#[derive(Debug)]
pub struct Balance {
    pub amount: f64,
    pub fee: f64,
}

impl Balance {
    pub fn with_fee(&self) -> f64 {
        self.amount * (1.0 - self.fee)
    }

    pub fn raw(&self) -> f64 {
        self.amount
    }
}

impl AmountCalculator {
    pub fn new(
        min_amount_threshold: f64,
        fee: f64,
    ) -> Option<AmountCalculator> {
        if fee >= 1.0 || fee < 0.0 {
            None
        } else {
            Some(AmountCalculator {
                min_amount_threshold,
                fee,
            })
        }
    }

    pub fn calculate(
        &self,
        direct_order: &Order,
        direct_coin_balance: f64,
        reversed_order: &Order,
        reversed_coin_balance: f64,
    ) -> Option<(f64, f64)> {
        let balance_amount = reversed_coin_balance.min(direct_coin_balance);
        let min_order_amount = direct_order.amount.min(reversed_order.amount);
        let result = if balance_amount <= min_order_amount {
            let max_amount = balance_amount * (1.0 - self.fee);
            if direct_order.price > reversed_order.price {
                (reversed_order.price * max_amount / direct_order.price, max_amount)
            } else {
                (max_amount, direct_order.price * max_amount / reversed_order.price)
            }
        } else {
            if direct_order.price > reversed_order.price {
                (reversed_order.price * min_order_amount / direct_order.price, min_order_amount)
            } else {
                (min_order_amount, direct_order.price * min_order_amount / reversed_order.price)
            }
        };
        match (result.0 > self.min_amount_threshold, result.1 > self.min_amount_threshold) {
            (true, true) => Some(result),
            _ => None,
        }
    }

    pub fn calculate_from_one_order(
        &self,
        order_amount: f64,
        balance: Balance,
    ) -> Option<Amount> {
        let balance_with_fee = balance.with_fee();
        if order_amount < balance_with_fee {
            if order_amount >= self.min_amount_threshold {
                Some(Amount::PriceBased(order_amount))
            } else {
                None
            }
        } else {
            if balance_with_fee >= self.min_amount_threshold {
                Some(Amount::BalanceBased(balance_with_fee))
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use agnostic::order::Order;
    use agnostic::trading_pair::{TradingPair, Target, Side, Coins};

    fn default_trading_pair(side: Side) -> TradingPair {
        TradingPair {
            coins: Coins::TonUsdt,
            side,
            target: Target::Market,
        }
    }

    #[test]
    fn prices() {
        let direct_pair = default_trading_pair(Side::Buy);
        let reversed_pair = direct_pair.clone().reversed_side();
        let mut direct_order = Order {
            trading_pair: direct_pair.clone(),
            amount: 100f64,
            price: 1f64
        };
        let mut reversed_order = Order {
            trading_pair: reversed_pair.clone(),
            amount: 110f64,
            price: 2f64,
        };
        let amount_calculator = AmountCalculator::new(0.1, 0.1).expect("Invalid fee");
        let calculated = amount_calculator.calculate(
            &direct_order,
            1000f64,
            &reversed_order,
            1000f64);
        assert_eq!(calculated, Some((100f64, 50f64)));
        {
            direct_order.price = 2f64;
            reversed_order.price = 1f64;
            let calculated = amount_calculator.calculate(
                &direct_order,
                1000f64,
                &reversed_order,
                1000f64);
            assert_eq!(calculated, Some((50f64, 100f64)));
            direct_order.price = 1f64;
            reversed_order.price = 2f64;
        }
    }

    #[test]
    fn amount_calculator() {
        let direct_pair = default_trading_pair(Side::Buy);
        let mut direct_order = Order {
            trading_pair: direct_pair.clone(),
            amount: 100f64,
            price: 1f64
        };
        let revesed_pair = direct_pair.clone().reversed_side();
        let mut revesed_order = Order {
            trading_pair: revesed_pair.clone(),
            amount: 100f64,
            price: 1f64,
        };
        let calculator = AmountCalculator::new(0.1, 0.1).unwrap();
        let amount = calculator.calculate(
            &direct_order,
            100f64,
            &revesed_order,
            100f64);
        assert_eq!(amount, Some((90f64, 90f64)));
        direct_order.amount = 0.1;
        let amount = calculator.calculate(
            &direct_order,
            100f64,
            &revesed_order,
            100f64);
        assert_eq!(amount, None);
        direct_order.amount = 100f64;
        revesed_order.amount = 0.0;
        let amount = calculator.calculate(
            &direct_order,
            100f64,
            &revesed_order,
            100f64);
        assert_eq!(amount, None);
        revesed_order.amount = 100f64;
        let amount = calculator.calculate(
            &direct_order,
            10f64,
            &revesed_order,
            20f64);
        assert_eq!(amount, Some((9.0, 9f64)));
    }

    #[test]
    fn one_order_test() {
        let calculator = AmountCalculator::new(0.1, 0.1).unwrap();
        let amount = calculator.calculate_from_one_order(
            100.0,
            Balance {
                amount: 100.0,
                fee: calculator.fee,
            }
        );
        assert_eq!(amount, Some(Amount::BalanceBased(90.0)))
    }
}

