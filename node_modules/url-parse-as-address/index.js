var url = require('url')
var assert = require('assert')

module.exports = parse
parse.format = format
parse.parse = parse

function parse (str, parseQueryString) {
  assert.equal(typeof str, 'string')
  var p = url.parse(str, parseQueryString)
  if (!p.slashes)
    p = url.parse('http://' + str, parseQueryString)
  else if (!p.protocol)
    p = url.parse('http:' + str, parseQueryString)

  return p
}

function format (str) {
  return url.format(parse(str))
}
