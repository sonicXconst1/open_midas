use agnostic::order::Order;

#[derive(Clone, Copy, Debug)]
pub struct LowAmountFilter {
    pub low_amount: f64,
}

impl LowAmountFilter {
    pub fn filter(&self, order: Vec<Order>) -> Vec<Order> {
        order.into_iter()
            .filter(|order| order.amount > self.low_amount)
            .collect()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn low_amount_filter() {
        let pair = agnostic::trading_pair::TradingPair {
            coins: agnostic::trading_pair::Coins::TonUsdt,
            side: agnostic::trading_pair::Side::Buy,
            target: agnostic::trading_pair::Target::Limit,
        };
        let orders = vec![
            Order {
                trading_pair: pair.clone(),
                price: 1f64,
                amount: 1f64,
            },
            Order {
                trading_pair: pair.clone(),
                price: 1f64,
                amount: 1f64,
            },
        ];
        let filter = LowAmountFilter {
            low_amount: 100f64,
        };
        assert_eq!(0, filter.filter(orders).len())
    }
}
