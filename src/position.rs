use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    json_types::U128,
    serde::Serialize,
    AccountId,
};

use crate::{errors::*, BASIS_POINT};

#[derive(Clone, Serialize, BorshDeserialize, BorshSerialize, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct Position {
    pub owner_id: AccountId,
    pub liquidity: f64,     // L
    pub token0_locked: f64, // x
    pub token1_locked: f64, // y
    pub tick_lower_bound_price: i32,
    pub tick_upper_bound_price: i32,
    pub sqrt_lower_bound_price: f64, // p_a
    pub sqrt_upper_bound_price: f64, // p_b
    pub is_active: bool,
    pub last_update: u64,
    pub rewards_for_time: u64,
    pub fees_earned_token0: u128,
    pub fees_earned_token1: u128,
}

impl Default for Position {
    fn default() -> Self {
        Position {
            owner_id: String::new(),
            liquidity: 0.0,
            token0_locked: 0.0,
            token1_locked: 0.0,
            tick_lower_bound_price: 0,
            tick_upper_bound_price: 0,
            sqrt_lower_bound_price: 0.0,
            sqrt_upper_bound_price: 0.0,
            is_active: false,
            last_update: 0,
            rewards_for_time: 0,
            fees_earned_token0: 0,
            fees_earned_token1: 0,
        }
    }
}

impl Position {
    pub fn new(
        owner_id: AccountId,
        token0_liquidity: Option<U128>,
        token1_liquidity: Option<U128>,
        lower_bound_price: f64,
        upper_bound_price: f64,
        sqrt_price: f64,
    ) -> Position {
        assert!(
            token0_liquidity.is_some() ^ token1_liquidity.is_some(),
            "{}",
            INCORRECT_TOKEN
        );
        assert!(lower_bound_price < upper_bound_price);
        let liquidity;
        let x;
        let y;
        let tick_lower_bound_price = sqrt_price_to_tick(lower_bound_price.sqrt());
        let tick_upper_bound_price = sqrt_price_to_tick(upper_bound_price.sqrt());
        let sqrt_lower_bound_price = tick_to_sqrt_price(tick_lower_bound_price);
        let sqrt_upper_bound_price = tick_to_sqrt_price(tick_upper_bound_price);
        if token0_liquidity.is_some() {
            let token0_liquidity: u128 = token0_liquidity.unwrap().into();
            x = token0_liquidity as f64;
            assert!(x > 0.0, "token0 liqudity cannot be 0");
            assert!(
                sqrt_price <= sqrt_upper_bound_price,
                "send token1 liquidity instead of token0"
            );
            if sqrt_lower_bound_price < sqrt_price && sqrt_price < sqrt_upper_bound_price {
                liquidity = get_liquidity_0(x, sqrt_price, sqrt_upper_bound_price);
            } else {
                liquidity = get_liquidity_0(x, sqrt_lower_bound_price, sqrt_upper_bound_price);
            }
            y = calculate_y(
                liquidity,
                sqrt_price,
                sqrt_lower_bound_price,
                sqrt_upper_bound_price,
            );
        } else {
            let token1_liquidity: u128 = token1_liquidity.unwrap().into();
            y = token1_liquidity as f64;
            assert!(y > 0.0, "token1 liqudity cannot be 0");
            assert!(
                sqrt_price >= sqrt_lower_bound_price,
                "send token0 liquidity instead of token1"
            );
            if sqrt_lower_bound_price <= sqrt_price && sqrt_price <= sqrt_upper_bound_price {
                liquidity = get_liquidity_1(y, sqrt_lower_bound_price, sqrt_price);
            } else {
                liquidity = get_liquidity_1(y, sqrt_lower_bound_price, sqrt_upper_bound_price);
            }
            x = calculate_x(
                liquidity,
                sqrt_price,
                sqrt_lower_bound_price,
                sqrt_upper_bound_price,
            );
        }
        Position {
            owner_id,
            liquidity,
            token0_locked: x,
            token1_locked: y,
            tick_lower_bound_price,
            tick_upper_bound_price,
            sqrt_lower_bound_price,
            sqrt_upper_bound_price,
            is_active: true,
            last_update: 0,
            rewards_for_time: 0,
            fees_earned_token0: 0,
            fees_earned_token1: 0,
        }
    }

    pub fn refresh(&mut self, sqrt_price: f64, current_timestamp: u64) {
        self.token0_locked = calculate_x(
            self.liquidity,
            sqrt_price,
            self.sqrt_lower_bound_price,
            self.sqrt_upper_bound_price,
        );
        self.token1_locked = calculate_y(
            self.liquidity,
            sqrt_price,
            self.sqrt_lower_bound_price,
            self.sqrt_upper_bound_price,
        );
        if self.is_active {
            self.rewards_for_time = current_timestamp - self.last_update;
        }
        self.is_active = self.is_active(sqrt_price);
        self.last_update = current_timestamp;
    }

    pub fn is_active(&self, sqrt_price: f64) -> bool {
        self.sqrt_lower_bound_price <= sqrt_price && self.sqrt_upper_bound_price >= sqrt_price
    }

    pub fn add_liquidity(
        &mut self,
        token0_liquidity: Option<U128>,
        token1_liquidity: Option<U128>,
        sqrt_price: f64,
    ) {
        assert!(
            token0_liquidity.is_some() ^ token1_liquidity.is_some(),
            "{}",
            INCORRECT_TOKEN
        );
        if token0_liquidity.is_some() {
            let token0_liquidity: u128 = token0_liquidity.unwrap().into();
            self.token0_locked += token0_liquidity as f64;
            assert!(
                sqrt_price <= self.sqrt_upper_bound_price,
                "send token1 liquidity instead of token0"
            );
            if self.sqrt_lower_bound_price < sqrt_price && sqrt_price < self.sqrt_upper_bound_price
            {
                self.liquidity =
                    get_liquidity_0(self.token0_locked, sqrt_price, self.sqrt_upper_bound_price);
            } else {
                self.liquidity = get_liquidity_0(
                    self.token0_locked,
                    self.sqrt_lower_bound_price,
                    self.sqrt_upper_bound_price,
                );
            }
            self.token1_locked = calculate_y(
                self.liquidity,
                sqrt_price,
                self.sqrt_lower_bound_price,
                self.sqrt_upper_bound_price,
            );
        } else {
            let token1_liquidity: u128 = token1_liquidity.unwrap().into();
            self.token1_locked += token1_liquidity as f64;
            assert!(
                sqrt_price >= self.sqrt_lower_bound_price,
                "send token0 liquidity instead of token0"
            );
            if self.sqrt_lower_bound_price <= sqrt_price
                && sqrt_price <= self.sqrt_upper_bound_price
            {
                self.liquidity =
                    get_liquidity_1(self.token1_locked, self.sqrt_lower_bound_price, sqrt_price);
            } else {
                self.liquidity = get_liquidity_1(
                    self.token1_locked,
                    self.sqrt_lower_bound_price,
                    self.sqrt_upper_bound_price,
                );
            }
            self.token0_locked = calculate_x(
                self.liquidity,
                sqrt_price,
                self.sqrt_lower_bound_price,
                self.sqrt_upper_bound_price,
            );
        }
    }

    pub fn remove_liquidity(
        &mut self,
        token0_liquidity: Option<U128>,
        token1_liquidity: Option<U128>,
        sqrt_price: f64,
    ) {
        assert!(
            token0_liquidity.is_some() ^ token1_liquidity.is_some(),
            "{}",
            INCORRECT_TOKEN
        );
        if token0_liquidity.is_some() {
            let token0_liquidity: u128 = token0_liquidity.unwrap().into();
            self.token0_locked -= token0_liquidity as f64;
            assert!(self.token0_locked > 0.0);
            assert!(
                sqrt_price <= self.sqrt_upper_bound_price,
                "send token1 liquidity instead of token0"
            );
            if self.sqrt_lower_bound_price < sqrt_price && sqrt_price < self.sqrt_upper_bound_price
            {
                self.liquidity =
                    get_liquidity_0(self.token0_locked, sqrt_price, self.sqrt_upper_bound_price);
            } else {
                self.liquidity = get_liquidity_0(
                    self.token0_locked,
                    self.sqrt_lower_bound_price,
                    self.sqrt_upper_bound_price,
                );
            }
            self.token1_locked = calculate_y(
                self.liquidity,
                sqrt_price,
                self.sqrt_lower_bound_price,
                self.sqrt_upper_bound_price,
            );
        } else {
            let token1_liquidity: u128 = token1_liquidity.unwrap().into();
            self.token1_locked -= token1_liquidity as f64;
            assert!(self.token1_locked > 0.0);
            assert!(
                sqrt_price >= self.sqrt_lower_bound_price,
                "send token0 liquidity instead of token0"
            );
            if self.sqrt_lower_bound_price <= sqrt_price
                && sqrt_price <= self.sqrt_upper_bound_price
            {
                self.liquidity =
                    get_liquidity_1(self.token1_locked, self.sqrt_lower_bound_price, sqrt_price);
            } else {
                self.liquidity = get_liquidity_1(
                    self.token1_locked,
                    self.sqrt_lower_bound_price,
                    self.sqrt_upper_bound_price,
                );
            }
            self.token0_locked = calculate_x(
                self.liquidity,
                sqrt_price,
                self.sqrt_lower_bound_price,
                self.sqrt_upper_bound_price,
            );
        }
    }
}

fn min(first: f64, second: f64) -> f64 {
    if first < second {
        first
    } else {
        second
    }
}

fn max(first: f64, second: f64) -> f64 {
    if first > second {
        first
    } else {
        second
    }
}

pub fn get_liquidity_0(x: f64, sa: f64, sb: f64) -> f64 {
    x * sa * sb / (sb - sa)
}

pub fn get_liquidity_1(y: f64, sa: f64, sb: f64) -> f64 {
    y / (sb - sa)
}

pub fn _get_liquidity(x: f64, y: f64, sp: f64, sa: f64, sb: f64) -> f64 {
    let liquidity;
    if sp <= sa {
        liquidity = get_liquidity_0(x, sa, sb);
    } else if sp < sb {
        let liquidity0 = get_liquidity_0(x, sp, sb);
        let liquidity1 = get_liquidity_1(y, sa, sp);
        liquidity = min(liquidity0, liquidity1)
    } else {
        liquidity = get_liquidity_1(y, sa, sb);
    }
    liquidity
}

pub fn calculate_x(l: f64, sp: f64, sa: f64, sb: f64) -> f64 {
    let sp = max(min(sp, sb), sa);
    l * (sb - sp) / (sp * sb)
}

pub fn calculate_y(l: f64, sp: f64, sa: f64, sb: f64) -> f64 {
    let sp = max(min(sp, sb), sa);
    l * (sp - sa)
}

pub fn _calculate_a1(l: f64, sp: f64, _sb: f64, _x: f64, y: f64) -> f64 {
    (sp - y / l).powf(2.0)
}

pub fn _calculate_a2(sp: f64, sb: f64, x: f64, y: f64) -> f64 {
    let sa = y / (sb * x) + sp - y / (sp * x);
    sa.powf(2.0)
}

pub fn _calculate_b1(l: f64, sp: f64, _sa: f64, x: f64, _y: f64) -> f64 {
    ((l * sp) / (l - sp * x)).powf(2.0)
}

pub fn _calculate_b2(sp: f64, sa: f64, x: f64, y: f64) -> f64 {
    let p = sp.powf(2.0);
    (sp * y / ((sa * sp - p) * x + y)).powf(2.0)
}

pub fn tick_to_sqrt_price(tick: i32) -> f64 {
    BASIS_POINT.powf(tick as f64 / 2.0)
}

pub fn sqrt_price_to_tick(sqrt_price: f64) -> i32 {
    (2.0 * sqrt_price.log(BASIS_POINT)).floor() as i32
}

pub fn _calculate_sp(l: f64, x: f64, sb: f64) -> f64 {
    (l * sb) / (x * sb + l)
}

#[cfg(test)]

mod test {
    use super::min;
    use crate::position::max;
    use crate::position::*;

    #[test]
    fn debug_info() {
        let p = 3227.02_f64;
        let a = 1626.3_f64;
        let b = 4846.3_f64;
        let x = 1_f64;
        let y = 5096.06_f64;
        println!("p = {}, a = {}, b = {}, x = {}, y = {}", p, a, b, x, y);
    }

    #[test]
    fn min_vault() {
        let first = 50_f64;
        let second = 100_f64;
        assert_eq!(min(first, second), 50_f64);
    }

    #[test]
    fn max_vault() {
        let first = 50_f64;
        let second = 100_f64;
        assert_eq!(max(first, second), 100_f64);
    }

    #[test]
    fn get_liquidity_0_test() {
        let sa = 1626.3_f64.powf(0.5);
        let sb = 4846.3_f64.powf(0.5);
        let x = 1_f64;
        let l_0 = get_liquidity_0(x, sa.powf(0.5), sb.powf(0.5)).floor();
        assert_eq!(l_0, 26.0);
        println!("sa = {}, sb = {}, x = {}, l_0 = {}", sa, sb, x, l_0);
    }

    #[test]
    fn get_liquidity_1_test() {
        let sa = 1626.3_f64.powf(0.5);
        let sb = 4846.3_f64.powf(0.5);
        let y = 5096.06_f64;
        let l_1 = get_liquidity_1(y, sa.powf(0.5), sb.powf(0.5)).floor();
        assert_eq!(l_1, 2556.0);
        println!("sa = {}, sb = {}, y = {}, l_1 = {}", sa, sb, y, l_1);
    }

    #[test]
    fn get_liquidity_test() {
        // At sp <= sa ((x * sa * sb)/(sb - sa))
        let mut sp = 1500.02_f64.powf(0.5);
        let mut sa = 3500.3_f64.powf(0.5);
        let mut sb = 1500.3_f64.powf(0.5);
        let mut x = 2_f64;
        let mut y = 5096.06_f64;
        let mut l = _get_liquidity(x, y, sp, sa, sb).floor();
        assert_eq!(l, -225.0);
        println!("sp <= sa, l = {}", l);
        // At sp < sb
        // min(get_liquidity_0, get_liquidity_1)
        // get_liquidity_0 = ((x * sa * sb)/(sb - sa))
        // get_liquidity_1 = y /(sb - sa)
        sp = 3227.02_f64.powf(0.5);
        sa = 3000.3_f64.powf(0.5);
        sb = 3800.3_f64.powf(0.5);
        x = 1_f64;
        y = 5096.06_f64;
        l = _get_liquidity(x, y, sp, sa, sb).floor();
        assert_eq!(l, 723.0);
        println!("sp < sb, l = {}", l);
        // At sa < sp > sb
        sp = 3600.02_f64.powf(0.5);
        sa = 3500.3_f64.powf(0.5);
        sb = 3000.3_f64.powf(0.5);
        x = 1_f64;
        y = 5096.06_f64;
        l = _get_liquidity(x, y, sp, sa, sb).floor();
        assert_eq!(l, -1162.0);
        println!(" sa < sp > sb, l = {}", l);
    }

    #[test]
    fn calculate_x_test() {
        let sp = 3227.02_f64.powf(0.5);
        let sa = 1626.3_f64.powf(0.5);
        let sb = 4846.3_f64.powf(0.5);
        let x = 1_f64;
        let y = 5096.06_f64;
        let l = _get_liquidity(x, y, sp, sa, sb);
        let x1 = calculate_x(l, sp, sa, sb);
        assert_eq!(x, 1.00);
        assert!(x == 1.0);
        println!("old x = {}, new x = {}", x, x1);
    }

    #[test]
    fn calculate_y_test() {
        let sp = 3227.02_f64.powf(0.5);
        let sa = 1626.3_f64.powf(0.5);
        let sb = 4846.3_f64.powf(0.5);
        let x = 1_f64;
        let y = 5096.06_f64;
        let l = _get_liquidity(x, y, sp, sa, sb);
        let y1 = calculate_y(l, sp, sa, sb);
        assert_eq!(y1.floor(), 5088.0);
        println!("old y = {}, new y = {}", y, y1);
    }

    #[test]
    fn calculate_a1_test() {
        let sp = 3227.02_f64.powf(0.5);
        let a = 1626.3_f64;
        let sa = 1626.3_f64.powf(0.5);
        let sb = 4846.3_f64.powf(0.5);
        let x = 1_f64;
        let y = 5096.06_f64;
        let l = _get_liquidity(x, y, sp, sa, sb);
        let a1 = _calculate_a1(l, sp, sb, x, y);
        assert_eq!(a1.floor(), 1624.0);
        println!("old a = {}, new a = {}", a, a1);
    }

    #[test]
    fn calculate_a2_test() {
        let sp = 3227.02_f64.powf(0.5);
        let a = 1626.3_f64;
        let sb = 4846.3_f64.powf(0.5);
        let x = 1_f64;
        let y = 5096.06_f64;
        let a2 = _calculate_a2(sp, sb, x, y);
        assert_eq!(a2.floor(), 1624.0);
        println!("old a = {}, new a delta = {}", a, a2);
    }

    #[test]
    fn calculate_b1_test() {
        let sp = 3227.02_f64.powf(0.5);
        let sa = 1626.3_f64.powf(0.5);
        let b = 4846.3_f64;
        let sb = 4846.3_f64.powf(0.5);
        let x = 1_f64;
        let y = 5096.06_f64;
        let l = _get_liquidity(x, y, sp, sa, sb);
        let b1 = _calculate_b1(l, sp, sa, x, y);
        assert_eq!(b1.floor(), 4846.0);
        println!("old b = {}, new b = {}", b, b1);
    }

    #[test]
    fn calculate_b2_test() {
        let sp = 3227.02_f64.powf(0.5);
        let sa = 1626.3_f64.powf(0.5);
        let b = 4846.3_f64;
        let x = 1_f64;
        let y = 5096.06_f64;
        let b2 = _calculate_b2(sp, sa, x, y);
        assert_eq!(b2.floor(), 4842.0);
        println!("old b = {}, new b delta = {}", b, b2);
    }

    #[test]
    fn open_position() {
        let position = Position::new(String::new(), Some(U128(50)), None, 25.0, 121.0, 10.0);
        assert!(position.owner_id == String::new(), "{}", _NO_VALID_OWNER_ID);
        assert!(
            position.token0_locked.floor() == 50.0,
            "{}",
            _TOKEN0_LIQUIDITY_DOESNT_MATCH
        );
        assert!(
            position.token1_locked == 27504.676564711368,
            "{}",
            _TOKEN1_LIQUIDITY_DOESNT_MATCH
        );
        assert!(
            position.liquidity == 5500.834197154125,
            "{}",
            _LIQUIDITY_DOESNT_MATCH
        );
        assert!(
            position.tick_lower_bound_price == 32190,
            "{}",
            _BAD_TICK_LOWER_BOUND_PRICE
        );
        assert!(
            position.tick_upper_bound_price == 47960,
            "{}",
            _BAD_TICK_UPPER_BOUND_PRICE
        );
        assert!(
            position.sqrt_lower_bound_price == 4.999908090496346,
            "{}",
            _BAD_SQRT_LOWER_BOUND_PRICE
        );
        assert!(
            position.sqrt_upper_bound_price == 10.999833188399927,
            "{}",
            _BAD_SQRT_LOWER_BOUND_PRICE
        );
    }

    #[test]
    fn open_position_less_than_lower_bound() {
        let position = Position::new(String::new(), Some(U128(50)), None, 121.0, 144.0, 10.0);
        assert!(position.owner_id == String::new(), "{}", _NO_VALID_OWNER_ID);
        assert!(
            position.token0_locked == 50.0,
            "{}",
            _TOKEN0_LIQUIDITY_DOESNT_MATCH
        );
        assert!(
            position.token1_locked == 0.0,
            "{}",
            _TOKEN1_LIQUIDITY_DOESNT_MATCH
        );
        assert!(
            position.liquidity == 6601.04186065018,
            "{}",
            _LIQUIDITY_DOESNT_MATCH
        );
        assert!(
            position.tick_lower_bound_price == 47960,
            "{}",
            _BAD_TICK_LOWER_BOUND_PRICE
        );
        assert!(
            position.tick_upper_bound_price == 49700,
            "{}",
            _BAD_TICK_UPPER_BOUND_PRICE
        );
        assert!(
            position.sqrt_lower_bound_price == 10.999833188399927,
            "{}",
            _BAD_SQRT_LOWER_BOUND_PRICE
        );
        assert!(
            position.sqrt_upper_bound_price == 11.99962930765891,
            "{}",
            _BAD_SQRT_LOWER_BOUND_PRICE
        );
    }

    #[test]
    fn open_position_more_than_upper_bound() {
        let position = Position::new(String::new(), None, Some(U128(50)), 121.0, 144.0, 13.0);
        assert!(position.owner_id == String::new(), "{}", _NO_VALID_OWNER_ID);
        assert!(
            position.token0_locked == 0.0,
            "{}",
            _TOKEN0_LIQUIDITY_DOESNT_MATCH
        );
        assert!(
            position.token1_locked == 50.0,
            "{}",
            _TOKEN1_LIQUIDITY_DOESNT_MATCH
        );
        assert!(
            position.liquidity == 50.010196115842504,
            "{}",
            _LIQUIDITY_DOESNT_MATCH
        );
        assert!(
            position.tick_lower_bound_price == 47960,
            "{}",
            _BAD_TICK_LOWER_BOUND_PRICE
        );
        assert!(
            position.tick_upper_bound_price == 49700,
            "{}",
            _BAD_TICK_UPPER_BOUND_PRICE
        );
        assert!(
            position.sqrt_lower_bound_price == 10.999833188399927,
            "{}",
            _BAD_SQRT_LOWER_BOUND_PRICE
        );
        assert!(
            position.sqrt_upper_bound_price == 11.99962930765891,
            "{}",
            _BAD_SQRT_LOWER_BOUND_PRICE
        );
    }

    #[should_panic(expected = "token0 liqudity cannot be 0")]
    #[test]
    fn open_position_wrong_order_x_zero() {
        let _position = Position::new(String::new(), Some(U128(0)), None, 121.0, 144.0, 11.5);
    }

    #[should_panic(expected = "send token1 liquidity instead of token0")]
    #[test]
    fn open_position_wrong_order_x_not_zero_higher_than_upper_bound() {
        let _position = Position::new(String::new(), Some(U128(1)), None, 121.0, 144.0, 13.0);
    }

    #[should_panic(expected = "token1 liqudity cannot be 0")]
    #[test]
    fn open_position_wrong_order_y_zero() {
        let _position = Position::new(String::new(), None, Some(U128(0)), 121.0, 144.0, 11.5);
    }

    #[should_panic(expected = "send token0 liquidity instead of token1")]
    #[test]
    fn open_position_wrong_order_y_not_zero_higher_than_upper_bound() {
        let _position = Position::new(String::new(), None, Some(U128(1)), 121.0, 144.0, 10.0);
    }

    #[test]
    fn open_position1() {
        let position = Position::new(
            String::new(),
            Some(U128(1000000000000000000)),
            None,
            900.0,
            1100.0,
            1000.0_f64.sqrt(),
        );
        assert!(position.token0_locked == 1000000000000000000.0);
        println!(
            "position.token1_real_liquidity = {}",
            position.token1_locked
        );
        assert!(position.token1_locked == 1103229672007021900000.0);
        assert!(position.liquidity == 679621668342898400000.0);
        println!(
            "position.sqrt_lower_bound_price = {}",
            position.sqrt_lower_bound_price
        );
        assert!(position.sqrt_lower_bound_price == 29.999476869794734);
        println!(
            "position.sqrt_upper_bound_price = {}",
            position.sqrt_upper_bound_price
        );
        assert!(position.sqrt_upper_bound_price == 33.16598911754618);
    }

    #[test]
    fn open_position2() {
        let position = Position::new(
            String::new(),
            Some(U128(1000000000000000000000000)),
            None,
            900.0,
            1100.0,
            1000.0_f64.sqrt(),
        );
        assert!(position.token0_locked == 1000000000000000000000000.0);
        assert!(position.token1_locked == 1103229672007021800000000000.0);
        assert!(position.liquidity == 679621668342898300000000000.0);
        println!(
            "position.sqrt_lower_bound_price = {}",
            position.sqrt_lower_bound_price
        );
        assert!(position.sqrt_lower_bound_price == 29.999476869794734);
        println!(
            "position.sqrt_upper_bound_price = {}",
            position.sqrt_upper_bound_price
        );
        assert!(position.sqrt_upper_bound_price == 33.16598911754618);
    }

    #[test]
    fn open_position3() {
        let position = Position::new(
            String::new(),
            Some(U128(1000000000000000000000000)),
            None,
            1000.0,
            1100.0,
            1000.0_f64.sqrt(),
        );
        assert!(position.token0_locked == 1000000000000000000000000.0);
        println!(
            "position.token1_real_liquidity = {}",
            position.token1_locked
        );
        assert!(position.token1_locked == 7102492217198050000000.0);
        assert!(position.liquidity == 679621668342898300000000000.0);
        println!(
            "position.sqrt_lower_bound_price = {}",
            position.sqrt_lower_bound_price
        );
        assert!(position.sqrt_lower_bound_price == 31.622766151027864);
        println!(
            "position.sqrt_upper_bound_price = {}",
            position.sqrt_upper_bound_price
        );
        assert!(position.sqrt_upper_bound_price == 33.16598911754618);
    }

    #[test]
    fn ticks1() {
        let tick = 500;
        let sqrt_price = tick_to_sqrt_price(tick);
        let new_tick = sqrt_price_to_tick(sqrt_price);
        assert!(tick == new_tick);
    }

    #[test]
    fn ticks2() {
        let sqrt_price = 10.0;
        let tick = sqrt_price_to_tick(sqrt_price);
        assert!(tick == 46054);
        let new_sqrt_price = tick_to_sqrt_price(tick + 1);
        println!("new_sqrt_price = {new_sqrt_price}");
        assert!(new_sqrt_price > sqrt_price);
        let new_tick = sqrt_price_to_tick(new_sqrt_price);
        assert!(new_tick > tick)
    }
}
