/**
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

const React = require("react");

class Footer extends React.Component {
  docUrl(doc) {
    const baseUrl = this.props.config.baseUrl;
    const docsUrl = this.props.config.docsUrl;
    const docsPart = `${docsUrl ? `${docsUrl}/` : ""}`;
    return `${baseUrl}${docsPart}${doc}`;
  }

  render() {
    return (
      <footer className="nav-footer" id="footer">
        <section className="sitemap">
          <a href={this.props.config.baseUrl} className="nav-home">
            {this.props.config.footerIcon && (
              <img
                src={this.props.config.baseUrl + this.props.config.footerIcon}
                alt={this.props.config.title}
                width="66"
                height="58"
              />
            )}
          </a>
          <div>
            <h5>User Docs</h5>
            <a href={this.docUrl("getting-started")}>Get Started</a>
            <a href={this.docUrl("faq")}>FAQ</a>
            <a href={this.docUrl("changelog")}>Changelog</a>
          </div>
          <div>
            <h5>PL Docs</h5>
            <a href={this.docUrl("compliance")}>BF Compliance</a>
            <a href={this.docUrl("optimisations")}>Optimisations</a>
            <a href={this.docUrl("testing")}>Testing</a>
          </div>
        </section>

        <section className="copyright">{this.props.config.copyright}</section>
      </footer>
    );
  }
}

module.exports = Footer;
