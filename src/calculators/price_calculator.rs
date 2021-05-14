pub struct PriceCalculator {
    pub profit: f64,
}

impl PriceCalculator {
    pub fn low(&self, price: f64) -> f64 {
        price * (1f64 - self.profit)
    }

    pub fn high(&self, price: f64) -> f64 {
        price * (1.0 + self.profit)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn low() {
        let calculator = PriceCalculator {
            profit: 0.1f64
        };
        let price = 100f64;
        let expected_price = 90f64;
        assert!((calculator.low(price) - expected_price).abs() < 1e-5)
    }

    #[test]
    fn high() {
        let calculator = PriceCalculator {
            profit: 0.1f64
        };
        let price = 100f64;
        let expected_price = 110.0;
        assert!((calculator.high(price) - expected_price).abs() < 1e-5)
    }
}
