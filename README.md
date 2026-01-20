<!-- LOGO -->
<p align="center">
  <a href="https://github.com/tileshq/">
    <img
      src="https://avatars.githubusercontent.com/u/210493283?s=400&u=1ee6e44b6a683b16bdb6e9e853c7ebd8c7fd4268&v=4"
      alt="Tiles Logo"
      width="128"
    />
  </a>
</p>

<h1 align="center">Tiles</h1>

<p align="center">
  Your private AI assistant with offline memory<br />
  <a href="https://tiles.run/download">Download</a> ·
  <a href="https://tiles.run/book">Documentation</a> ·
  <a href="#about">About</a> ·
  <a href="CONTRIBUTING.md">Contributing</a>
</p>

---

## Getting Started

There are two primary ways to work with Tiles, depending on whether you are an end user or a developer.

## Tiles CLI

Tiles is a private AI assistant for everyday use. The CLI is the fastest way to get started and will feel familiar if you have used tools like Ollama or LM Studio.

Install and run:

```bash
curl -fsSL https://tiles.run/install.sh | sh
tiles run
```

This launches the recommended default memory model for your device.

## Tilekit SDK

Tilekit is a Modelfile-based Rust SDK that powers all model deployment in Tiles.

It provides:
- A Modelfile specification for defining and sharing models
- Fast, efficient local deployment across consumer platforms
- Model composition and chaining, with MIR support currently in development

Tilekit is the foundation for building custom local models and agent experiences within Tiles.

## Download

Get the latest release from the Tiles website:  
https://tiles.run/download

## Documentation

Full documentation is available in the Tiles Book:  
https://tiles.run/book

## About

Our mission is to bring privacy technology to everyone.

Tiles Privacy emerged from the [User & Agents](https://userandagents.com) community with a simple principle: software should understand you without taking anything from you.

We build privacy-first engineering that does not compromise on usability. Identity and memory belong together, and Tiles enables you to own both through a personal user agent that runs on your device.

Tiles is designed for privacy-conscious users who want intelligence without handing over their memory to centralized providers. Our first offerings are:
- A private AI assistant for everyday use
- A Modelfile-based SDK for building and customizing local models and agents

We are seeking design partners for training workloads that align with our goal of a verifiable privacy perimeter. Contact us at hello@tiles.run.

## Contributing

Ideas, issues, and pull requests are welcome.

Start here:
- [Contributing to Tiles](CONTRIBUTING.md)
- [Developing Tiles](HACKING.md)

## License

This project is dual-licensed under MIT and Apache 2.0:

- MIT License: https://github.com/tileshq/tilekit/blob/main/LICENSE-MIT.txt
- Apache License 2.0: https://github.com/tileshq/tilekit/blob/main/LICENSE-APACHE.txt

You may choose either license, or both. Apache 2.0 is included for its explicit patent protections.
