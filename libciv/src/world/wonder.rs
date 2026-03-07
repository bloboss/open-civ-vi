use crate::YieldBundle;

pub trait NaturalWonder: std::fmt::Debug {
    fn name(&self) -> &'static str;
    fn yields(&self) -> YieldBundle;
    fn appeal_bonus(&self) -> i32 { 0 }
}
