// Surge Ignore
// ============
//
// The default things we ignore when you
// deploy to Surge. https://surge.sh
//
// 1. We really don’t want your `.git` directory
// 2. We don’t need any dotfiles, like `.DS_Store`
// 3. Vim temp files. See Issue #1.
// 4. We don’t want `node_modules/` since those should
//    already be compiled in somehow. Developers can opt-out
//    of this by adding `!node_modules/` to their `.surgeignore`.
// 5. Same goes for Bower as npm.

module.exports = [
  ".git",             // [1]
  ".*",               // [2]
  "*.*~",             // [3]
  "node_modules",     // [4]
  "bower_components"  // [5]
]
