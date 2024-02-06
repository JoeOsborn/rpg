# Role-Playing Game

Start with your graphical roguelike if you want, or use the starter code I've provided; either way, we're going to move off in a bit of a different direction.  This assignment is to make a role-playing game, but that can mean different things.  The purpose of this assignment is to get practice implementing game UI elements, and letting input button presses do different things in different situations.

We'll use a point system again, but this time the steps will be bigger.  Points will come in two categories: style and structure.  You need five points this time around for full marks.

In terms of strategy, you can draw UI elements using a `NineSlice` object (stored in `Game::window`) and draw text using `BitFont` (`Game::font`).  I have an example of a very trivial dialog system that draws a text box if there is an active dialog index, and you're welcome to build off of that.

If you're on the map, keyboard inputs should move you around and the map should be drawn.  If you're in battle, you could either draw the battle scene on top of the map (and covering up parts of it) or against a black background (i.e., don't draw the map when you're in battle).

This project is primarily about *refactoring* and *expanding on* an example.

Don't worry too much about creating the perfect menu system.  It's probably fine to have e.g. fields for your `PlayerStatsDisplay`, `BattleEnemyDisplay`, `BattleOptionChooser`, `EnemyPicker`, etc with booleans describing whether they're active or not.  Drawing one of those could render a box (with nineslice) and some text (with bitfont) and an extra sprite to be the "cursor".  You can either cram mode-specific data into the `GameMode::Battle` or `GameMode::Map` enum variants, or you can put them into `Game` behind an `Option` where appropriate.

It might make sense to make a Menu struct to wrap up the dimensions, the options to pick from, its depth offset (higher is "further away" or "goes underneath more things"), which nineslice to use, and whether the menu is active.  It could have a function to draw itself into the transform and UV slices and return how many sprites it used.
