use serde::{Deserialize, Serialize};
use ulid::Ulid;

macro_rules! define_api_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        pub struct $name(pub Ulid);

        impl $name {
            pub fn from_ulid(ulid: Ulid) -> Self {
                Self(ulid)
            }

            pub fn as_ulid(&self) -> Ulid {
                self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }

        // OpenAPI schema: every wire ID newtype serializes as a 26-char
        // Crockford-base32 ULID string, so describe it as a string with
        // an explicit `format: ulid` and a deterministic example.
        #[cfg(feature = "openapi")]
        impl utoipa::PartialSchema for $name {
            fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
                use utoipa::openapi::schema::{ObjectBuilder, SchemaType, Type};
                ObjectBuilder::new()
                    .schema_type(SchemaType::Type(Type::String))
                    .format(Some(utoipa::openapi::SchemaFormat::Custom("ulid".to_string())))
                    .description(Some(concat!(
                        stringify!($name),
                        " — Crockford-base32 ULID (26 chars). Wire form is the bare ULID string."
                    )))
                    .examples([serde_json::Value::String(
                        "01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
                    )])
                    .into()
            }
        }

        #[cfg(feature = "openapi")]
        impl utoipa::ToSchema for $name {
            fn name() -> std::borrow::Cow<'static, str> {
                std::borrow::Cow::Borrowed(stringify!($name))
            }
        }
    };
}

define_api_id!(CityId);
define_api_id!(UnitId);
define_api_id!(CivId);
define_api_id!(TechId);
define_api_id!(CivicId);
define_api_id!(GovernmentId);
define_api_id!(PolicyId);
define_api_id!(ReligionId);
define_api_id!(WonderId);
define_api_id!(GreatPersonId);
define_api_id!(PromotionId);
define_api_id!(ImprovementId);
define_api_id!(ResourceId);
define_api_id!(RoadId);
define_api_id!(AgreementId);
define_api_id!(GrievanceId);
define_api_id!(GovernorId);
define_api_id!(BeliefId);
define_api_id!(VictoryId);
define_api_id!(UnitTypeId);
define_api_id!(DistrictTypeId);
define_api_id!(BuildingId);
define_api_id!(TradeRouteId);
define_api_id!(EraId);
define_api_id!(TerrainId);
define_api_id!(FeatureId);
define_api_id!(EdgeFeatureId);
define_api_id!(NaturalWonderId);
define_api_id!(ProjectId);
define_api_id!(GameId);
define_api_id!(CivTemplateId);
