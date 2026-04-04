//! Game room logic: action dispatch and turn resolution.

use libciv::civ::ProductionItem;
use libciv::game::recalculate_visibility;
use libciv::{CivId, RulesEngine, TurnEngine};

use crate::types::messages::GameAction;

use crate::server::state::GameRoom;

impl GameRoom {
    /// Apply a player action, validating ownership.
    /// Returns Ok(()) on success, or an error message.
    pub fn apply_action(&mut self, civ_id: CivId, action: &GameAction) -> Result<(), String> {
        match action {
            GameAction::MoveUnit { unit, to } => {
                let uid = to_libciv_unit_id(*unit);
                let u = self.state.unit(uid).ok_or("unit not found")?;
                if u.owner != civ_id { return Err("not your unit".into()); }
                let dest = to_libciv_coord(*to);
                match self.rules.move_unit(&self.state, uid, dest) {
                    Ok(diff) | Err(libciv::game::RulesError::InsufficientMovement(diff)) => {
                        for delta in &diff.deltas {
                            if let libciv::game::StateDelta::UnitMoved { unit, to, cost, .. } = delta
                                && let Some(u) = self.state.unit_mut(*unit)
                            {
                                u.coord = *to;
                                u.movement_left = u.movement_left.saturating_sub(*cost);
                            }
                        }
                        recalculate_visibility(&mut self.state, civ_id);
                        Ok(())
                    }
                    Err(e) => Err(format!("{e:?}")),
                }
            }
            GameAction::Attack { attacker, defender } => {
                let atk = to_libciv_unit_id(*attacker);
                let def = to_libciv_unit_id(*defender);
                let u = self.state.unit(atk).ok_or("attacker not found")?;
                if u.owner != civ_id { return Err("not your unit".into()); }
                self.rules.attack(&mut self.state, atk, def)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            }
            GameAction::FoundCity { settler, name } => {
                let sid = to_libciv_unit_id(*settler);
                let u = self.state.unit(sid).ok_or("settler not found")?;
                if u.owner != civ_id { return Err("not your unit".into()); }
                self.rules.found_city(&mut self.state, sid, name.clone())
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            }
            GameAction::PlaceImprovement { coord, improvement } => {
                let c = to_libciv_coord(*coord);
                let imp = to_libciv_improvement(*improvement);
                self.rules.place_improvement(&mut self.state, civ_id, c, imp, None)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            }
            GameAction::QueueProduction { city, item } => {
                let city_id = to_libciv_city_id(*city);
                let city = self.state.cities.iter_mut()
                    .find(|c| c.id == city_id)
                    .ok_or("city not found")?;
                if city.owner != civ_id { return Err("not your city".into()); }
                let prod_item = to_libciv_production_item(item);
                city.production_queue.push_back(prod_item);
                Ok(())
            }
            GameAction::CancelProduction { city, index } => {
                let city_id = to_libciv_city_id(*city);
                let city = self.state.cities.iter_mut()
                    .find(|c| c.id == city_id)
                    .ok_or("city not found")?;
                if city.owner != civ_id { return Err("not your city".into()); }
                if *index < city.production_queue.len() {
                    city.production_queue.remove(*index);
                    if *index == 0 { city.production_stored = 0; }
                }
                Ok(())
            }
            GameAction::EstablishTradeRoute { trader, destination } => {
                let tid = to_libciv_unit_id(*trader);
                let dest = to_libciv_city_id(*destination);
                let u = self.state.unit(tid).ok_or("trader not found")?;
                if u.owner != civ_id { return Err("not your unit".into()); }
                self.rules.establish_trade_route(&mut self.state, tid, dest)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            }
            GameAction::QueueResearch { tech } => {
                let tech_id = to_libciv_tech_id(*tech);
                let civ = self.state.civilizations.iter_mut()
                    .find(|c| c.id == civ_id)
                    .ok_or("civ not found")?;
                // Add to research queue if not already present.
                if !civ.research_queue.iter().any(|tp| tp.tech_id == tech_id) {
                    civ.research_queue.push_back(libciv::civ::TechProgress {
                        tech_id, progress: 0, boosted: false,
                    });
                }
                Ok(())
            }
            GameAction::QueueCivic { civic } => {
                let civic_id = to_libciv_civic_id(*civic);
                let civ = self.state.civilizations.iter_mut()
                    .find(|c| c.id == civ_id)
                    .ok_or("civ not found")?;
                if civ.civic_in_progress.is_none() {
                    civ.civic_in_progress = Some(libciv::civ::CivicProgress {
                        civic_id, progress: 0, inspired: false,
                    });
                }
                Ok(())
            }
            GameAction::AssignCitizen { city, tile, lock } => {
                let city_id = to_libciv_city_id(*city);
                let coord = to_libciv_coord(*tile);
                self.rules.assign_citizen(&mut self.state, city_id, coord, *lock)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            }
            GameAction::UnassignCitizen { city, tile } => {
                let city_id = to_libciv_city_id(*city);
                let coord = to_libciv_coord(*tile);
                let city = self.state.cities.iter_mut()
                    .find(|c| c.id == city_id)
                    .ok_or("city not found")?;
                if city.owner != civ_id { return Err("not your city".into()); }
                city.worked_tiles.retain(|t| *t != coord);
                city.locked_tiles.remove(&coord);
                Ok(())
            }
            GameAction::DeclareWar { target } => {
                let target_id = to_libciv_civ_id(*target);
                self.rules.declare_war(&mut self.state, civ_id, target_id)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            }
            GameAction::MakePeace { target } => {
                let target_id = to_libciv_civ_id(*target);
                self.rules.make_peace(&mut self.state, civ_id, target_id)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            }
            GameAction::AssignPolicy { policy } => {
                let pid = to_libciv_policy_id(*policy);
                self.rules.assign_policy(&mut self.state, civ_id, pid)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            }
            GameAction::FoundPantheon { belief } => {
                let bid = to_libciv_belief_id(*belief);
                self.rules.found_pantheon(&mut self.state, civ_id, bid)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            }
            GameAction::FoundReligion { prophet, name, beliefs } => {
                let pid = to_libciv_unit_id(*prophet);
                let u = self.state.unit(pid).ok_or("prophet not found")?;
                if u.owner != civ_id { return Err("not your unit".into()); }
                let bids: Vec<libciv::BeliefId> = beliefs.iter().map(|b| to_libciv_belief_id(*b)).collect();
                self.rules.found_religion(&mut self.state, pid, name.clone(), bids)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            }
            GameAction::SpreadReligion { unit } => {
                let uid = to_libciv_unit_id(*unit);
                let u = self.state.unit(uid).ok_or("unit not found")?;
                if u.owner != civ_id { return Err("not your unit".into()); }
                self.rules.spread_religion(&mut self.state, uid)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            }
            GameAction::TheologicalCombat { attacker, defender } => {
                let atk = to_libciv_unit_id(*attacker);
                let def = to_libciv_unit_id(*defender);
                let u = self.state.unit(atk).ok_or("attacker not found")?;
                if u.owner != civ_id { return Err("not your unit".into()); }
                self.rules.theological_combat(&mut self.state, atk, def)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            }
            GameAction::PurchaseWithFaith { city, item } => {
                let cid = to_libciv_city_id(*city);
                // Parse item string as faith purchase.
                let purchase_item = libciv::game::FaithPurchaseItem::Unit(
                    match item.as_str() {
                        "Missionary" => "Missionary",
                        "Apostle" => "Apostle",
                        _ => return Err(format!("unknown faith purchase: {item}")),
                    }
                );
                self.rules.purchase_with_faith(&mut self.state, civ_id, cid, purchase_item)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            }
        }
    }

    /// Resolve the current turn: advance game state, run AI, reset movement.
    pub fn resolve_turn(&mut self) {
        let engine = TurnEngine::new();
        engine.process_turn(&mut self.state, &self.rules);

        // Reset movement for all units.
        for unit in &mut self.state.units {
            unit.movement_left = unit.max_movement;
        }

        // AI turns.
        for (ai_civ, agent) in &self.ai_agents {
            use libciv::ai::Agent;
            agent.take_turn(&mut self.state, &self.rules);
            recalculate_visibility(&mut self.state, *ai_civ);
        }

        // Recalculate visibility for all human players.
        for slot in &self.players {
            recalculate_visibility(&mut self.state, slot.civ_id);
        }

        // Reset turn submission flags.
        for slot in &mut self.players {
            slot.submitted_turn = false;
        }
    }

    /// Check if all human players have submitted their turn.
    pub fn all_submitted(&self) -> bool {
        self.players.iter().all(|s| s.submitted_turn)
    }
}

// ── ID conversion helpers (API -> libciv) ────────────────────────────────────

fn to_libciv_coord(c: crate::types::coord::HexCoord) -> libhexgrid::coord::HexCoord {
    libhexgrid::coord::HexCoord::from_qr(c.q, c.r)
}

fn to_libciv_unit_id(id: crate::types::ids::UnitId) -> libciv::UnitId {
    libciv::UnitId::from_ulid(id.as_ulid())
}

fn to_libciv_city_id(id: crate::types::ids::CityId) -> libciv::CityId {
    libciv::CityId::from_ulid(id.as_ulid())
}

fn to_libciv_civ_id(id: crate::types::ids::CivId) -> libciv::CivId {
    libciv::CivId::from_ulid(id.as_ulid())
}

fn to_libciv_tech_id(id: crate::types::ids::TechId) -> libciv::TechId {
    libciv::TechId::from_ulid(id.as_ulid())
}

fn to_libciv_civic_id(id: crate::types::ids::CivicId) -> libciv::CivicId {
    libciv::CivicId::from_ulid(id.as_ulid())
}

fn to_libciv_belief_id(id: crate::types::ids::BeliefId) -> libciv::BeliefId {
    libciv::BeliefId::from_ulid(id.as_ulid())
}

fn to_libciv_policy_id(id: crate::types::ids::PolicyId) -> libciv::PolicyId {
    libciv::PolicyId::from_ulid(id.as_ulid())
}

fn to_libciv_improvement(i: crate::types::enums::BuiltinImprovement) -> libciv::world::improvement::BuiltinImprovement {
    use libciv::world::improvement::BuiltinImprovement as I;
    use crate::types::enums::BuiltinImprovement as A;
    match i {
        A::Farm => I::Farm, A::Mine => I::Mine, A::LumberMill => I::LumberMill,
        A::TradingPost => I::TradingPost, A::Fort => I::Fort, A::Airstrip => I::Airstrip,
        A::MissileSilo => I::MissileSilo,
        A::Quarry => I::Quarry, A::Plantation => I::Plantation,
        A::Camp => I::Camp, A::FishingBoats => I::FishingBoats,
        A::Pasture => I::Pasture,
        A::Sphinx => I::Sphinx, A::Stepwell => I::Stepwell,
        A::OilWell => I::OilWell, A::OffshoreOilRig => I::OffshoreOilRig,
        A::BeachResort => I::BeachResort, A::Chateau => I::Chateau,
        A::ColossalHead => I::ColossalHead, A::GreatWall => I::GreatWall,
        A::Kurgan => I::Kurgan, A::Mission => I::Mission,
        A::RomanFort => I::RomanFort, A::Ziggurat => I::Ziggurat,
    }
}

fn to_libciv_production_item(item: &crate::types::enums::ProductionItemView) -> ProductionItem {
    use crate::types::enums::ProductionItemView as P;
    match item {
        P::Unit(id) => ProductionItem::Unit(libciv::UnitTypeId::from_ulid(id.as_ulid())),
        P::Building(id) => ProductionItem::Building(libciv::BuildingId::from_ulid(id.as_ulid())),
        P::District(d) => ProductionItem::District(to_libciv_district(*d)),
        P::Wonder(id) => ProductionItem::Wonder(libciv::WonderId::from_ulid(id.as_ulid())),
    }
}

fn to_libciv_district(d: crate::types::enums::BuiltinDistrict) -> libciv::civ::district::BuiltinDistrict {
    use libciv::civ::district::BuiltinDistrict as D;
    use crate::types::enums::BuiltinDistrict as A;
    match d {
        A::Campus => D::Campus, A::TheaterSquare => D::TheaterSquare,
        A::CommercialHub => D::CommercialHub, A::Harbor => D::Harbor,
        A::HolySite => D::HolySite, A::Encampment => D::Encampment,
        A::IndustrialZone => D::IndustrialZone,
        A::EntertainmentComplex => D::EntertainmentComplex,
        A::WaterPark => D::WaterPark,
        A::Aqueduct => D::Aqueduct,
        A::Dam => D::Dam,
        A::Canal => D::Canal,
        A::Aerodrome => D::Aerodrome, A::Neighborhood => D::Neighborhood,
        A::Spaceport => D::Spaceport, A::CityCenter => D::CityCenter,
        A::Lavra => D::Lavra, A::Mbanza => D::Mbanza,
        A::StreetCarnival => D::StreetCarnival, A::RoyalNavyDockyard => D::RoyalNavyDockyard,
    }
}
