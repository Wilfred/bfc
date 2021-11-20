var path   = require('path');
var Ignore = require('fstream-ignore');

describe('surge-ignore', function() {
  it('Should be cool.', function(done) {
    done();
  });
  it('Should ignore ~ files.', function() {
    var stuff = Ignore({
      path: path.resolve('./fixtures'),
      ignoreFiles: ['.surgeignore']
    })
    console.log(stuff);
  })

});
