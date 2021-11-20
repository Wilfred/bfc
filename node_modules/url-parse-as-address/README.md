# url-parse-as-address

Parse a url assuming `http` if no protocol or `//` is provided.

Useful for parsing things like `foo.com` and not interpreting it as a
path.

## USAGE

```javascript
var parse = require('url-parse-as-address')
var assert = require('assert')

assert.deepEqual(parse('foo.com:1234/x?y=z#a=b'),
  { protocol: 'http:',
    slashes: true,
    auth: null,
    host: 'foo.com:1234',
    port: '1234',
    hostname: 'foo.com',
    hash: '#a=b',
    search: '?y=z',
    query: 'y=z',
    pathname: '/x',
    path: '/x?y=z',
    href: 'http://foo.com:1234/x?y=z#a=b' })

assert.deepEqual(parse('foo.com:1234/x?y=z#a=b', true),
  { protocol: 'http:',
    slashes: true,
    auth: null,
    host: 'foo.com:1234',
    port: '1234',
    hostname: 'foo.com',
    hash: '#a=b',
    search: '?y=z',
    query: { y: 'z' },
    pathname: '/x',
    path: '/x?y=z',
    href: 'http://foo.com:1234/x?y=z#a=b' })

// etc
```

By default this lib assumes `http:` is the protocol if none is
provided, because that's what web browsers do.

## API

* `parse(url, parseQueryString)` Parse a string to object.

* `parse.parse(..)` Same function, for symmetry to `url` builtin

* `parse.format(url)` Like `url.format()`

## SEE ALSO

* https://iojs.org/api/url.html
* https://nodejs.org/api/url.html
