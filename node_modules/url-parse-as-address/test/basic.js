var test = require('tap').test
var parse = require('../')

var tests = {
  'x': 'http://x/',
  'foo.com': 'http://foo.com/',
  'a@b:123/c': 'http://a@b:123/c'
}

Object.keys(tests).forEach(function (c) {
  test(c, function (t) {
    t.equal(parse(c).href, tests[c])
    t.equal(parse(tests[c]).href, tests[c])
    t.equal(parse.format(c), tests[c])
    t.end()
  })
})
