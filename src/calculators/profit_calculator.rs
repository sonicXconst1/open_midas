use agnostic::order::Order;

#[derive(Default, Copy, Clone, Debug)]
pub struct ProfitCalculator {
}

impl ProfitCalculator {
    pub fn calculate(
        &self,
        direct_order: &Order,
        reversed_order: &Order,
    ) -> Option<f64> {
        let direct = direct_order.price;
        let reversed = reversed_order.price;
        self.evaluate(direct, reversed)
    }

    pub fn evaluate(&self, sell_price: f64, buy_price: f64) -> Option<f64> {
        if sell_price >= buy_price {
            Some(1.0 - buy_price / sell_price)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use agnostic::order::Order;
    use agnostic::trading_pair::{TradingPair, Target, Side, Coins};

    #[test]
    fn profit_calculator() {
        let direct_pair = TradingPair {
            coins: Coins::TonUsdt,
            side: Side::Buy,
            target: Target::Market,
        };
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
        let calculator = ProfitCalculator::default();
        let amount = calculator.calculate(&direct_order, &revesed_order);
        assert_eq!(amount, Some(0.0));
        revesed_order.price = 2f64;
        let amount = calculator.calculate(&direct_order, &revesed_order);
        assert_eq!(amount, None);
        revesed_order.price = 1f64;
        direct_order.price = 2f64;
        let amount = calculator.calculate(&direct_order, &revesed_order);
        assert_eq!(amount, Some(0.5));
        direct_order.price = 1f64;
    }
}
