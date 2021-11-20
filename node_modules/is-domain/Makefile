build: components lib/index.js
	@component build --dev

clean:
	@rm -fr build components node_modules

components: component.json
	@component install --dev

node_modules: package.json
	@npm install

test: test-node test-component

test-component: node_modules build
	@component test browser

test-node: node_modules
	@./node_modules/.bin/mocha --reporter spec

.PHONY: clean
.PHONY: test
.PHONY: test-component
.PHONY: test-node