var Moniker = require('moniker');

var names = Moniker.generator([Moniker.adjective, Moniker.noun]);

console.log(names.choose());
