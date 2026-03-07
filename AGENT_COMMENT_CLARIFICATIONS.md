# Comment clarifications

## Library Consolidation

Make it so.

## Trait Overuse

I suppose an alternative option is that we use rust as a configuration language.
I would rather not use XML just since it is difficult to edit cleanly, but
perhaps we could use some other configuration language like yaml or, even more
ideally, a typed configuration language that we can check ahead of time. Does
toml fulfill these needs? If not, we could try something like
[Dhall](https://github.com/Nadrieril/dhall-rust)
though I note that the repo is no longer maintained. Perhaps toml is the best
regular data language?

Lets discuss this further, flag it when we are talking about the architecture
next

## HexTile/HexEdge relationship

Perhaps we say that a HexEdge has a coordinate and then a HexDirection, and we
define equivalence for the HexEdge operation, constraining generation so that we
never generate two edges for the same tile. I'm open to also having the
coordinate system just a simple offset from the tile one or being a fractional
version.

## Elevation, Vision

make it so

## MovementProfile

You're right about the equation. I suppose you're right that we could just add
it to the movement profile, so let's do that for the moment.

## strings

Let's just do this now, if we defer for too long then lifetimes will eat our
souls.

## CityStatus

Make it so.

## Puppet City

Neat, I was unaware of that. We keep it and clarify it.

## CityStatus

Yes, let's break these into CityOwnership , CityCondition enums

## WallLevel

As we are discussing the shortcomings of the overuse of traits. Perhaps we
should keep it as an enum for the moment and just add fields for the defense
modifier and current hp. We could also do a struct, which might more
realistically provide an interface for mods in the future.

## CityState

Let's make it so, keeping the specific CityState mechanics in a separate file
but the actual city existence being in the City file.

## Terrain Preferences

engage

## Lua string

go forth and make it so

## Leader abilities, strategic resources

Make. it. so.

## Diplomacy

Lets go forth

## Districts

Go forth, since we are increasing the coupling and taking a more wholistic view
of the game state this should not be too complicated.

## Era

Yeah, sounds good
