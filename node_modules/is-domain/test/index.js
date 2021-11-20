
try {
  var domain = require('is-domain');
} catch (e) {
  var domain = require('..');
}

var assert = require('assert');

describe('is-domain', function(){
  var valids = [
    'google.com',
    'google.com.au',
    'google.co.uk',
    'google.co.gov.edu'
  ];

  var invalids = [
    'http:',
    'http://google',
    '.google.com',
    '.google.co.uk',
    '.google.co.gov.edu',
    'http://google.',
    'http://google.com/something',
    'http://google.com?q=query',
    'http://google.com#hash',
    'http://google.com/something?q=query#hash',
    'https://d1f4470da51b49289906b3d6cbd65074@app.getsentry.com/13176',
    'http://localhost',
    'postgres://u:p@example.com:5702/db',
    'redis://:123@174.129.42.52:13271',
    'mongodb://u:p@example.com:10064/db',
    'ws://chat.example.com/games',
    'wss://secure.example.com/biz',
    'http://localhost:4000',
    'http://localhost:342/a/path'
  ];

  describe('valid', function(){
    valids.forEach(function(string){
      it(string, function(){
        assert(domain(string));
      });
    });
  });

  describe('invalid', function(){
    invalids.forEach(function(string){
      it(string, function(){
        assert(!domain(string));
      });
    });
  });
});