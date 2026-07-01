//! Game room logic: action dispatch and turn resolution.

use libciv::civ::ProductionItem;
use libciv::game::diff::{GameStateDiff, StateDelta};
use libciv::game::recalculate_visibility;
use libciv::{CivId, RulesEngine, TurnEngine};

use open4x_protocol::v1::messages::GameAction;

use crate::server::state::{GameRoom, NotificationKind, NotificationRecord, NotificationTarget};

/// Walk every StateDelta from the resolved turn and append notification
/// records to the per-civ buffer. Player-visible events only — internal
/// bookkeeping deltas (CitizenAssigned, GoldChanged etc.) are omitted to
/// avoid spamming the feed.
fn emit_notifications_from_diff(room: &mut GameRoom, diff: &GameStateDiff) {
    let civ_ids: Vec<CivId> = room.players.iter().map(|s| s.civ_id).collect();
    let turn = room.state.turn;

    for delta in &diff.deltas {
        match delta {
            StateDelta::TechResearched { civ, tech } => {
                if civ_ids.contains(civ) {
                    let api_civ = api_civ_id(*civ);
                    room.notifications.push(
                        api_civ,
                        NotificationRecord {
                            id: String::new(),
                            turn,
                            kind: NotificationKind::Accent,
                            category: "research",
                            title: "Research complete".into(),
                            desc: format!("{tech} researched · choose next →"),
                            target: Some(NotificationTarget {
                                screen: "tech".into(),
                                q: None,
                                r: None,
                            }),
                        },
                    );
                }
            }
            StateDelta::CivicCompleted { civ, civic } => {
                if civ_ids.contains(civ) {
                    let api_civ = api_civ_id(*civ);
                    room.notifications.push(
                        api_civ,
                        NotificationRecord {
                            id: String::new(),
                            turn,
                            kind: NotificationKind::Accent,
                            category: "civic",
                            title: "Civic unlocked".into(),
                            desc: format!("{civic} complete"),
                            target: Some(NotificationTarget {
                                screen: "civics".into(),
                                q: None,
                                r: None,
                            }),
                        },
                    );
                }
            }
            StateDelta::EurekaTriggered { civ, tech } => {
                if civ_ids.contains(civ) {
                    let api_civ = api_civ_id(*civ);
                    room.notifications.push(
                        api_civ,
                        NotificationRecord {
                            id: String::new(),
                            turn,
                            kind: NotificationKind::Good,
                            category: "research",
                            title: "Eureka!".into(),
                            desc: format!("Boost toward {tech}"),
                            target: Some(NotificationTarget {
                                screen: "tech".into(),
                                q: None,
                                r: None,
                            }),
                        },
                    );
                }
            }
            StateDelta::InspirationTriggered { civ, civic } => {
                if civ_ids.contains(civ) {
                    let api_civ = api_civ_id(*civ);
                    room.notifications.push(
                        api_civ,
                        NotificationRecord {
                            id: String::new(),
                            turn,
                            kind: NotificationKind::Good,
                            category: "civic",
                            title: "Inspiration!".into(),
                            desc: format!("Boost toward {civic}"),
                            target: Some(NotificationTarget {
                                screen: "civics".into(),
                                q: None,
                                r: None,
                            }),
                        },
                    );
                }
            }
            StateDelta::CityFounded {
                city: _,
                coord,
                owner,
            } => {
                if civ_ids.contains(owner) {
                    let api_civ = api_civ_id(*owner);
                    room.notifications.push(
                        api_civ,
                        NotificationRecord {
                            id: String::new(),
                            turn,
                            kind: NotificationKind::Good,
                            category: "city",
                            title: "City founded".into(),
                            desc: format!("at ({}, {})", coord.q, coord.r),
                            target: Some(NotificationTarget {
                                screen: "hud".into(),
                                q: Some(coord.q),
                                r: Some(coord.r),
                            }),
                        },
                    );
                }
            }
            StateDelta::CityCaptured {
                new_owner,
                old_owner,
                ..
            } => {
                if civ_ids.contains(new_owner) {
                    let api_civ = api_civ_id(*new_owner);
                    room.notifications.push(
                        api_civ,
                        NotificationRecord {
                            id: String::new(),
                            turn,
                            kind: NotificationKind::Good,
                            category: "military",
                            title: "City captured".into(),
                            desc: "An enemy city has fallen.".into(),
                            target: Some(NotificationTarget {
                                screen: "overview".into(),
                                q: None,
                                r: None,
                            }),
                        },
                    );
                }
                if civ_ids.contains(old_owner) {
                    let api_civ = api_civ_id(*old_owner);
                    room.notifications.push(
                        api_civ,
                        NotificationRecord {
                            id: String::new(),
                            turn,
                            kind: NotificationKind::Bad,
                            category: "military",
                            title: "City lost".into(),
                            desc: "We lost a city.".into(),
                            target: Some(NotificationTarget {
                                screen: "overview".into(),
                                q: None,
                                r: None,
                            }),
                        },
                    );
                }
            }
            StateDelta::PopulationGrew {
                city: _,
                new_population,
            } => {
                // Attribute to all civs that own the city — find from state.
                for civ in &civ_ids {
                    let api_civ = api_civ_id(*civ);
                    room.notifications.push(
                        api_civ,
                        NotificationRecord {
                            id: String::new(),
                            turn,
                            kind: NotificationKind::Neutral,
                            category: "city",
                            title: "Population growth".into(),
                            desc: format!("A city grew to {new_population}"),
                            target: Some(NotificationTarget {
                                screen: "overview".into(),
                                q: None,
                                r: None,
                            }),
                        },
                    );
                }
            }
            StateDelta::BuildingCompleted { city: _, building } => {
                for civ in &civ_ids {
                    let api_civ = api_civ_id(*civ);
                    room.notifications.push(
                        api_civ,
                        NotificationRecord {
                            id: String::new(),
                            turn,
                            kind: NotificationKind::Good,
                            category: "production",
                            title: "Building complete".into(),
                            desc: format!("{building} finished"),
                            target: Some(NotificationTarget {
                                screen: "city".into(),
                                q: None,
                                r: None,
                            }),
                        },
                    );
                }
            }
            StateDelta::WonderBuilt { civ, wonder, .. } => {
                if civ_ids.contains(civ) {
                    let api_civ = api_civ_id(*civ);
                    room.notifications.push(
                        api_civ,
                        NotificationRecord {
                            id: String::new(),
                            turn,
                            kind: NotificationKind::Accent,
                            category: "wonder",
                            title: "Wonder built".into(),
                            desc: format!("{wonder} complete"),
                            target: Some(NotificationTarget {
                                screen: "city".into(),
                                q: None,
                                r: None,
                            }),
                        },
                    );
                }
            }
            StateDelta::DiplomacyChanged {
                civ_a,
                civ_b,
                new_status,
            } => {
                let mention = |me: &CivId, other: &CivId| {
                    if civ_ids.contains(me) {
                        let api_me = api_civ_id(*me);
                        let api_other = api_civ_id(*other);
                        let kind = match new_status {
                            libciv::civ::diplomacy::DiplomaticStatus::War => NotificationKind::Bad,
                            libciv::civ::diplomacy::DiplomaticStatus::Alliance => {
                                NotificationKind::Good
                            }
                            _ => NotificationKind::Warn,
                        };
                        Some((
                            api_me,
                            NotificationRecord {
                                id: String::new(),
                                turn,
                                kind,
                                category: "diplomacy",
                                title: format!("Relations: {new_status:?}"),
                                desc: format!("with civ {api_other}"),
                                target: Some(NotificationTarget {
                                    screen: "dipl".into(),
                                    q: None,
                                    r: None,
                                }),
                            },
                        ))
                    } else {
                        None
                    }
                };
                if let Some((c, rec)) = mention(civ_a, civ_b) {
                    room.notifications.push(c, rec);
                }
                if let Some((c, rec)) = mention(civ_b, civ_a) {
                    room.notifications.push(c, rec);
                }
            }
            StateDelta::EventFired { civ, event_id } => {
                if civ_ids.contains(civ)
                    && let Some(def) = libciv::game::rules::events::event_def(event_id)
                {
                    let api_civ = api_civ_id(*civ);
                    room.notifications.push(
                        api_civ,
                        NotificationRecord {
                            id: String::new(),
                            turn,
                            kind: event_kind_to_notif(def.notif.kind),
                            category: def.notif.category,
                            title: def.notif.title.to_string(),
                            desc: def.notif.desc.to_string(),
                            target: None,
                        },
                    );
                }
            }
            // Bookkeeping deltas we deliberately don't surface as notifications
            _ => {}
        }
    }
}

fn api_civ_id(c: CivId) -> open4x_protocol::v1::ids::CivId {
    open4x_protocol::v1::ids::CivId::from_ulid(c.as_ulid())
}

/// Map the libciv-side event tone onto the server `NotificationKind`.
fn event_kind_to_notif(kind: libciv::game::rules::events::EventKind) -> NotificationKind {
    use libciv::game::rules::events::EventKind as E;
    match kind {
        E::Accent => NotificationKind::Accent,
        E::Good => NotificationKind::Good,
        E::Warn => NotificationKind::Warn,
        E::Bad => NotificationKind::Bad,
        E::Neutral => NotificationKind::Neutral,
    }
}

impl GameRoom {
    /// Apply a player action, validating ownership.
    /// Returns Ok(()) on success, or an error message.
    pub fn apply_action(&mut self, civ_id: CivId, action: &GameAction) -> Result<(), String> {
        match action {
            GameAction::MoveUnit { unit, to } => {
                let uid = to_libciv_unit_id(*unit);
                let u = self.state.unit(uid).ok_or("unit not found")?;
                if u.owner != civ_id {
                    return Err("not your unit".into());
                }
                let dest = to_libciv_coord(*to);
                match self.rules.move_unit(&self.state, uid, dest) {
                    Ok(diff) | Err(libciv::game::RulesError::InsufficientMovement(diff)) => {
                        for delta in &diff.deltas {
                            if let libciv::game::StateDelta::UnitMoved { unit, to, cost, .. } =
                                delta
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
                if u.owner != civ_id {
                    return Err("not your unit".into());
                }
                self.rules
                    .attack(&mut self.state, atk, def)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            }
            GameAction::FoundCity { settler, name } => {
                let sid = to_libciv_unit_id(*settler);
                let u = self.state.unit(sid).ok_or("settler not found")?;
                if u.owner != civ_id {
                    return Err("not your unit".into());
                }
                self.rules
                    .found_city(&mut self.state, sid, name.clone())
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            }
            GameAction::PlaceImprovement { coord, improvement } => {
                let c = to_libciv_coord(*coord);
                let imp = to_libciv_improvement(*improvement);
                self.rules
                    .place_improvement(&mut self.state, civ_id, c, imp, None)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            }
            GameAction::QueueProduction { city, item } => {
                let city_id = to_libciv_city_id(*city);
                let city = self
                    .state
                    .cities
                    .iter_mut()
                    .find(|c| c.id == city_id)
                    .ok_or("city not found")?;
                if city.owner != civ_id {
                    return Err("not your city".into());
                }
                let prod_item = to_libciv_production_item(item);
                city.production_queue.push_back(prod_item);
                Ok(())
            }
            GameAction::CancelProduction { city, index } => {
                let city_id = to_libciv_city_id(*city);
                let city = self
                    .state
                    .cities
                    .iter_mut()
                    .find(|c| c.id == city_id)
                    .ok_or("city not found")?;
                if city.owner != civ_id {
                    return Err("not your city".into());
                }
                if *index < city.production_queue.len() {
                    city.production_queue.remove(*index);
                    if *index == 0 {
                        city.production_stored = 0;
                    }
                }
                Ok(())
            }
            GameAction::EstablishTradeRoute {
                trader,
                destination,
            } => {
                let tid = to_libciv_unit_id(*trader);
                let dest = to_libciv_city_id(*destination);
                let u = self.state.unit(tid).ok_or("trader not found")?;
                if u.owner != civ_id {
                    return Err("not your unit".into());
                }
                self.rules
                    .establish_trade_route(&mut self.state, tid, dest)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            }
            GameAction::QueueResearch { tech } => {
                let tech_id = to_libciv_tech_id(*tech);
                let civ = self
                    .state
                    .civilizations
                    .iter_mut()
                    .find(|c| c.id == civ_id)
                    .ok_or("civ not found")?;
                // Add to research queue if not already present.
                if !civ.research_queue.iter().any(|tp| tp.tech_id == tech_id) {
                    civ.research_queue.push_back(libciv::civ::TechProgress {
                        tech_id,
                        progress: 0,
                        boosted: false,
                    });
                }
                Ok(())
            }
            GameAction::CancelResearch => {
                let civ = self
                    .state
                    .civilizations
                    .iter_mut()
                    .find(|c| c.id == civ_id)
                    .ok_or("civ not found")?;
                // Drop the front of the queue (active research). Partial
                // progress is discarded. Idempotent: no-op on empty queue.
                civ.research_queue.pop_front();
                Ok(())
            }
            GameAction::CancelCivic => {
                let civ = self
                    .state
                    .civilizations
                    .iter_mut()
                    .find(|c| c.id == civ_id)
                    .ok_or("civ not found")?;
                // Clear the active civic. Idempotent: no-op when None.
                civ.civic_in_progress = None;
                Ok(())
            }
            GameAction::ChangeGovernment { name } => {
                // Find the requested government in the registry. Use the
                // engine's static `&str` form so we set the same fields
                // OneShotEffect::AdoptGovernment does.
                let new_gov = self
                    .state
                    .governments
                    .iter()
                    .find(|g| g.name == name.as_str())
                    .cloned()
                    .ok_or_else(|| format!("unknown government: {name:?}"))?;

                // Unlock check: civ must have completed the prereq civic.
                let civ_idx = self
                    .state
                    .civilizations
                    .iter()
                    .position(|c| c.id == civ_id)
                    .ok_or("civ not found")?;
                let unlocked = self.state.civilizations[civ_idx]
                    .unlocked_governments
                    .contains(&new_gov.name);
                if !unlocked {
                    return Err(format!("government {} is not unlocked", new_gov.name));
                }

                // Mirror OneShotEffect::AdoptGovernment: evict policies
                // that no longer fit the new slot configuration.
                let mut mil = new_gov.slots.military as i32;
                let mut eco = new_gov.slots.economic as i32;
                let mut dip = new_gov.slots.diplomatic as i32;
                let mut wc = new_gov.slots.wildcard as i32;
                let active = self.state.civilizations[civ_idx].active_policies.clone();
                let mut kept = Vec::new();
                for pid in active {
                    let policy_type = self
                        .state
                        .policies
                        .iter()
                        .find(|p| p.id == pid)
                        .map(|p| p.policy_type);
                    use libciv::PolicyType as PT;
                    let fits = match policy_type {
                        Some(PT::Military) if mil > 0 => {
                            mil -= 1;
                            true
                        }
                        Some(PT::Economic) if eco > 0 => {
                            eco -= 1;
                            true
                        }
                        Some(PT::Diplomatic) if dip > 0 => {
                            dip -= 1;
                            true
                        }
                        Some(PT::Wildcard) if wc > 0 => {
                            wc -= 1;
                            true
                        }
                        _ => false,
                    };
                    if fits {
                        kept.push(pid);
                    }
                }
                self.state.civilizations[civ_idx].active_policies = kept;
                self.state.civilizations[civ_idx].current_government = Some(new_gov.id);
                self.state.civilizations[civ_idx].current_government_name = Some(new_gov.name);
                Ok(())
            }
            GameAction::QueueCivic { civic } => {
                let civic_id = to_libciv_civic_id(*civic);
                let civ = self
                    .state
                    .civilizations
                    .iter_mut()
                    .find(|c| c.id == civ_id)
                    .ok_or("civ not found")?;
                if civ.civic_in_progress.is_none() {
                    civ.civic_in_progress = Some(libciv::civ::CivicProgress {
                        civic_id,
                        progress: 0,
                        inspired: false,
                    });
                }
                Ok(())
            }
            GameAction::AssignCitizen { city, tile, lock } => {
                let city_id = to_libciv_city_id(*city);
                let coord = to_libciv_coord(*tile);
                self.rules
                    .assign_citizen(&mut self.state, city_id, coord, *lock)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            }
            GameAction::UnassignCitizen { city, tile } => {
                let city_id = to_libciv_city_id(*city);
                let coord = to_libciv_coord(*tile);
                let city = self
                    .state
                    .cities
                    .iter_mut()
                    .find(|c| c.id == city_id)
                    .ok_or("city not found")?;
                if city.owner != civ_id {
                    return Err("not your city".into());
                }
                city.worked_tiles.retain(|t| *t != coord);
                city.locked_tiles.remove(&coord);
                Ok(())
            }
            GameAction::DeclareWar { target } => {
                let target_id = to_libciv_civ_id(*target);
                self.rules
                    .declare_war(&mut self.state, civ_id, target_id)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            }
            GameAction::MakePeace { target } => {
                let target_id = to_libciv_civ_id(*target);
                self.rules
                    .make_peace(&mut self.state, civ_id, target_id)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            }
            GameAction::AssignPolicy { policy } => {
                let pid = to_libciv_policy_id(*policy);
                self.rules
                    .assign_policy(&mut self.state, civ_id, pid)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            }
            GameAction::FoundPantheon { belief } => {
                let bid = to_libciv_belief_id(*belief);
                self.rules
                    .found_pantheon(&mut self.state, civ_id, bid)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            }
            GameAction::FoundReligion {
                prophet,
                name,
                beliefs,
            } => {
                let pid = to_libciv_unit_id(*prophet);
                let u = self.state.unit(pid).ok_or("prophet not found")?;
                if u.owner != civ_id {
                    return Err("not your unit".into());
                }
                let bids: Vec<libciv::BeliefId> =
                    beliefs.iter().map(|b| to_libciv_belief_id(*b)).collect();
                self.rules
                    .found_religion(&mut self.state, pid, name.clone(), bids)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            }
            GameAction::SpreadReligion { unit } => {
                let uid = to_libciv_unit_id(*unit);
                let u = self.state.unit(uid).ok_or("unit not found")?;
                if u.owner != civ_id {
                    return Err("not your unit".into());
                }
                self.rules
                    .spread_religion(&mut self.state, uid)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            }
            GameAction::TheologicalCombat { attacker, defender } => {
                let atk = to_libciv_unit_id(*attacker);
                let def = to_libciv_unit_id(*defender);
                let u = self.state.unit(atk).ok_or("attacker not found")?;
                if u.owner != civ_id {
                    return Err("not your unit".into());
                }
                self.rules
                    .theological_combat(&mut self.state, atk, def)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            }
            GameAction::RenameCity { city, name } => {
                let city_id = to_libciv_city_id(*city);
                let trimmed = name.trim();
                if trimmed.is_empty() {
                    return Err("city name must not be empty".into());
                }
                if trimmed.chars().count() > 64 {
                    return Err("city name must be 64 characters or fewer".into());
                }
                let city = self
                    .state
                    .cities
                    .iter_mut()
                    .find(|c| c.id == city_id)
                    .ok_or("city not found")?;
                if city.owner != civ_id {
                    return Err("not your city".into());
                }
                city.name = trimmed.to_string();
                Ok(())
            }
            GameAction::AssignCityFocus { city, focus } => {
                let city_id = to_libciv_city_id(*city);
                let city = self
                    .state
                    .cities
                    .iter_mut()
                    .find(|c| c.id == city_id)
                    .ok_or("city not found")?;
                if city.owner != civ_id {
                    return Err("not your city".into());
                }
                city.focus = to_libciv_city_focus(*focus);
                Ok(())
            }
            GameAction::PurchaseWithFaith { city, item } => {
                let cid = to_libciv_city_id(*city);
                // Parse item string as faith purchase.
                let purchase_item = libciv::game::FaithPurchaseItem::Unit(match item.as_str() {
                    "Missionary" => "Missionary",
                    "Apostle" => "Apostle",
                    _ => return Err(format!("unknown faith purchase: {item}")),
                });
                self.rules
                    .purchase_with_faith(&mut self.state, civ_id, cid, purchase_item)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            }
        }
    }

    /// Resolve the current turn: advance game state, run AI, reset movement.
    pub fn resolve_turn(&mut self) {
        let engine = TurnEngine::new();
        let diff = engine.process_turn(&mut self.state, &self.rules);

        // Convert StateDeltas into per-civ NotificationRecords.
        emit_notifications_from_diff(self, &diff);

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

fn to_libciv_coord(c: open4x_protocol::v1::coord::HexCoord) -> libhexgrid::coord::HexCoord {
    libhexgrid::coord::HexCoord::from_qr(c.q, c.r)
}

fn to_libciv_unit_id(id: open4x_protocol::v1::ids::UnitId) -> libciv::UnitId {
    libciv::UnitId::from_ulid(id.as_ulid())
}

fn to_libciv_city_id(id: open4x_protocol::v1::ids::CityId) -> libciv::CityId {
    libciv::CityId::from_ulid(id.as_ulid())
}

fn to_libciv_civ_id(id: open4x_protocol::v1::ids::CivId) -> libciv::CivId {
    libciv::CivId::from_ulid(id.as_ulid())
}

fn to_libciv_tech_id(id: open4x_protocol::v1::ids::TechId) -> libciv::TechId {
    libciv::TechId::from_ulid(id.as_ulid())
}

fn to_libciv_civic_id(id: open4x_protocol::v1::ids::CivicId) -> libciv::CivicId {
    libciv::CivicId::from_ulid(id.as_ulid())
}

fn to_libciv_belief_id(id: open4x_protocol::v1::ids::BeliefId) -> libciv::BeliefId {
    libciv::BeliefId::from_ulid(id.as_ulid())
}

fn to_libciv_policy_id(id: open4x_protocol::v1::ids::PolicyId) -> libciv::PolicyId {
    libciv::PolicyId::from_ulid(id.as_ulid())
}

fn to_libciv_city_focus(f: open4x_protocol::v1::enums::CityFocus) -> libciv::civ::CityFocus {
    use libciv::civ::CityFocus as L;
    use open4x_protocol::v1::enums::CityFocus as A;
    match f {
        A::Default => L::Default,
        A::Food => L::Food,
        A::Production => L::Production,
        A::Gold => L::Gold,
        A::Science => L::Science,
        A::Culture => L::Culture,
        A::Faith => L::Faith,
    }
}

pub(crate) fn from_libciv_city_focus(
    f: libciv::civ::CityFocus,
) -> open4x_protocol::v1::enums::CityFocus {
    use libciv::civ::CityFocus as L;
    use open4x_protocol::v1::enums::CityFocus as A;
    match f {
        L::Default => A::Default,
        L::Food => A::Food,
        L::Production => A::Production,
        L::Gold => A::Gold,
        L::Science => A::Science,
        L::Culture => A::Culture,
        L::Faith => A::Faith,
    }
}

fn to_libciv_improvement(
    i: open4x_protocol::v1::enums::BuiltinImprovement,
) -> libciv::world::improvement::BuiltinImprovement {
    use libciv::world::improvement::BuiltinImprovement as I;
    use open4x_protocol::v1::enums::BuiltinImprovement as A;
    match i {
        A::Farm => I::Farm,
        A::Mine => I::Mine,
        A::LumberMill => I::LumberMill,
        A::TradingPost => I::TradingPost,
        A::Fort => I::Fort,
        A::Airstrip => I::Airstrip,
        A::MissileSilo => I::MissileSilo,
        A::Quarry => I::Quarry,
        A::Plantation => I::Plantation,
        A::Camp => I::Camp,
        A::FishingBoats => I::FishingBoats,
        A::Pasture => I::Pasture,
        A::Sphinx => I::Sphinx,
        A::Stepwell => I::Stepwell,
        A::OilWell => I::OilWell,
        A::OffshoreOilRig => I::OffshoreOilRig,
        A::BeachResort => I::BeachResort,
        A::Chateau => I::Chateau,
        A::ColossalHead => I::ColossalHead,
        A::GreatWall => I::GreatWall,
        A::Kurgan => I::Kurgan,
        A::Mission => I::Mission,
        A::RomanFort => I::RomanFort,
        A::Ziggurat => I::Ziggurat,
        A::SolarFarm => I::SolarFarm,
        A::WindFarm => I::WindFarm,
        A::OffshoreWindFarm => I::OffshoreWindFarm,
        A::GeothermalPlant => I::GeothermalPlant,
        A::Seastead => I::Seastead,
        A::MountainTunnel => I::MountainTunnel,
        A::SkiResort => I::SkiResort,
    }
}

fn to_libciv_production_item(
    item: &open4x_protocol::v1::enums::ProductionItemView,
) -> ProductionItem {
    use open4x_protocol::v1::enums::ProductionItemView as P;
    match item {
        P::Unit(id) => ProductionItem::Unit(libciv::UnitTypeId::from_ulid(id.as_ulid())),
        P::Building(id) => ProductionItem::Building(libciv::BuildingId::from_ulid(id.as_ulid())),
        P::District(d) => ProductionItem::District(to_libciv_district(*d)),
        P::Wonder(id) => ProductionItem::Wonder(libciv::WonderId::from_ulid(id.as_ulid())),
        P::Project(id) => ProductionItem::Project(libciv::ProjectId::from_ulid(id.as_ulid())),
    }
}

fn to_libciv_district(
    d: open4x_protocol::v1::enums::BuiltinDistrict,
) -> libciv::civ::district::BuiltinDistrict {
    use libciv::civ::district::BuiltinDistrict as D;
    use open4x_protocol::v1::enums::BuiltinDistrict as A;
    match d {
        A::Campus => D::Campus,
        A::TheaterSquare => D::TheaterSquare,
        A::CommercialHub => D::CommercialHub,
        A::Harbor => D::Harbor,
        A::HolySite => D::HolySite,
        A::Encampment => D::Encampment,
        A::IndustrialZone => D::IndustrialZone,
        A::EntertainmentComplex => D::EntertainmentComplex,
        A::WaterPark => D::WaterPark,
        A::Aqueduct => D::Aqueduct,
        A::Dam => D::Dam,
        A::Canal => D::Canal,
        A::Aerodrome => D::Aerodrome,
        A::Neighborhood => D::Neighborhood,
        A::Spaceport => D::Spaceport,
        A::CityCenter => D::CityCenter,
        A::Lavra => D::Lavra,
        A::Mbanza => D::Mbanza,
        A::StreetCarnival => D::StreetCarnival,
        A::RoyalNavyDockyard => D::RoyalNavyDockyard,
    }
}
