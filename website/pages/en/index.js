/**
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

const React = require("react");

const CompLibrary = require("../../core/CompLibrary.js");

const MarkdownBlock = CompLibrary.MarkdownBlock; /* Used to read markdown */
const Container = CompLibrary.Container;
const GridBlock = CompLibrary.GridBlock;

class HomeSplash extends React.Component {
  render() {
    const { siteConfig, language = "" } = this.props;
    const { baseUrl, docsUrl } = siteConfig;
    const docsPart = `${docsUrl ? `${docsUrl}/` : ""}`;
    const docUrl = (doc) => `${baseUrl}${docsPart}${doc}`;

    const SplashContainer = (props) => (
      <div className="homeContainer">
        <div className="homeSplashFade">
          <div className="wrapper homeWrapper">{props.children}</div>
        </div>
      </div>
    );

    const Logo = (props) => (
      <div className="projectLogo">
        <img src={props.img_src} alt="bfc logo" />
      </div>
    );

    const ProjectTitle = (props) => (
      <h2 className="projectTitle">
        {props.title}
        <small>{props.tagline}</small>
      </h2>
    );

    const PromoSection = (props) => (
      <div className="section promoSection">
        <div className="promoRow">
          <div className="pluginRowBlock">{props.children}</div>
        </div>
      </div>
    );

    const Button = (props) => (
      <div className="pluginWrapper buttonWrapper">
        <a className="button" href={props.href} target={props.target}>
          {props.children}
        </a>
      </div>
    );

    return (
      <SplashContainer>
        <Logo img_src={`${baseUrl}img/logo.png`} />
        <div className="inner">
          <ProjectTitle tagline={siteConfig.tagline} title={siteConfig.title} />
          <PromoSection>
            <Button href={docUrl("getting-started")}>Get Started</Button>
            <Button href="https://github.com/Wilfred/bfc">
              Source on GitHub
            </Button>
          </PromoSection>
        </div>
      </SplashContainer>
    );
  }
}

class Index extends React.Component {
  render() {
    const { config: siteConfig, language = "" } = this.props;
    const { baseUrl } = siteConfig;

    const Block = (props) => (
      <Container
        padding={["bottom", "top"]}
        id={props.id}
        background={props.background}
      >
        <GridBlock
          contents={props.children}
          layout={props.layout}
        />
      </Container>
    );

    const Optimising = () => (
      <Block id="try" background="light">
        {[
          {
            content:
            "bfc uses traditional compiler techniques to reduce runtime and memory usage.\n\nbfc includes compile-time evaluation, dead code elimination, and constant folding.\n\n[Learn more about optimisations](/docs/optimisations).",
            image: `${baseUrl}img/undraw_stepping_up.svg`,
            imageAlign: "left",
            title: "a fast compiler for a silly language",
          },
        ]}
      </Block>
    );

    const Overengineered = () => (
      <Block background="dark">
        {[
          {
            content:
              "An elaborate IR with position-preserving optimisations.\n\n[Extensive testing](/docs/testing), even testing idempotence and observational equivalence of optimisations.\n\nColoured [code diagnostics](/docs/getting-started#diagnostics) with position highlighting.\n\nGratuitous website.",
            image: `${baseUrl}img/undraw_researching.svg`,
            imageAlign: "right",
            title: "utterly over-engineered",
          },
        ]}
      </Block>
    );

    return (
      <div>
        <HomeSplash siteConfig={siteConfig} language={language} />
        <div className="mainContainer">
          <Optimising />
          <Overengineered />
        </div>
      </div>
    );
  }
}

module.exports = Index;
