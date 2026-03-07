use crate::YieldBundle;
pub use crate::ResourceCategory;

pub trait Resource: std::fmt::Debug {
    fn name(&self) -> &'static str;
    fn category(&self) -> ResourceCategory;
    fn base_yields(&self) -> YieldBundle;
    /// Tech required to see this resource (None = always visible).
    fn reveal_tech(&self) -> Option<&'static str> { None }
}
