# Diplomacy

## Diplomatic Status

Relations between civilizations are tracked via `DiplomaticRelation`:

```rust
struct DiplomaticRelation {
    civ_a: CivId,
    civ_b: CivId,
    status: DiplomaticStatus,
    grievances_a_against_b: Vec<GrievanceRecord>,
    grievances_b_against_a: Vec<GrievanceRecord>,
    active_agreements: Vec<Box<dyn Agreement>>,
    turns_at_war: u32,
}
```

### Status Levels

| Status | Description |
|--------|-------------|
| War | Active military conflict |
| Denounced | Public condemnation (-reputation) |
| Neutral | Default state |
| Friendly | Positive relations |
| Alliance | Military/economic cooperation |

### Actions

- `declare_war(target)` -- changes status to War, generates grievance
- `make_peace(target)` -- changes status to Neutral, resets war counter

## Grievance System

Grievances track diplomatic offenses between civilizations:

```rust
struct GrievanceRecord {
    grievance_id: GrievanceId,
    description: &'static str,
    amount: i32,
    visibility: GrievanceVisibility,
    recorded_turn: u32,
}
```

### Built-in Grievance Triggers

| Trigger | Amount | Description |
|---------|--------|-------------|
| Declared War | 30 | Declaring war on another civ |
| Pillage | 5 | Pillaging improvements |
| Captured City | 20 | Taking a city by force |

### Visibility

Grievances have visibility levels:
- `Public` -- visible to all civilizations
- `RequiresSpy` -- only visible with espionage
- `RequiresAlliance` -- only visible to allies

### Opinion Score

`opinion_score_a_toward_b()` computes a numeric opinion value based on:
- Sum of grievances (negative)
- Status modifiers
- Agreement bonuses (positive)

This score influences AI decision-making via the `Agenda` trait.

## Agreements

The `Agreement` trait represents bilateral agreements:

```rust
trait Agreement {
    fn id(&self) -> AgreementId;
    fn name(&self) -> &'static str;
    fn duration_turns(&self) -> u32;
    fn is_expired(&self, current_turn: u32) -> bool;
}
```

Agreements have a fixed duration and expire automatically. They are stored on `DiplomaticRelation.active_agreements`.
