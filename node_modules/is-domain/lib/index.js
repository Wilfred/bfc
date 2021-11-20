/**
 * Expose `isDomain`.
 */

module.exports = isDomain;

/**
 * Matcher.
 */

var matcher = /^[a-zA-Z0-9_-]+\.[.a-zA-Z0-9_-]+$/;

/**
 * Loosely validate a domain `string`.
 *
 * @param {String} string
 * @return {Boolean}
 */

function isDomain(string){
  return matcher.test(string);
}