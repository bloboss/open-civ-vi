# Comments on Effect Planning (@PLAN_EFFECTS.md)

- New OneShotEffect seems sound
- Permanent modifiers for civs may be better accrued from their various
  enablers. I do not like the idea of adding a field that is intricately linked
  to the other fields of a civ. We should remove this and default back to the
  old way where the modifiers is a method `Civilization::get_modifiers(&mut
  self) -> Vec<Modifier>;`
- Perhaps we should also remove this idea of a distinct cross civilization
  effect, since those are already calculated at turn boundaries by the rules
  engine.
