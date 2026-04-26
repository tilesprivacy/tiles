<!-- LOGO -->

<p align="center">
  <a href="https://github.com/tilesprivacy/">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="https://tiles.run/realdark.png" />
      <source media="(prefers-color-scheme: light)" srcset="https://tiles.run/reallight.png" />
      <img src="https://github.com/user-attachments/assets/1c7848de-33af-47ed-9fb2-a361c096a44d" alt="Tiles Logo" width="128" />
    </picture>
  </a>
</p>

<h1 align="center">Tiles</h1>

<p align="center">
  Local-first private AI for everyday use.<br />
  <a href="#getting-started">Getting Started</a> ·
  <a href="https://tiles.run/book">Documentation</a> ·
  <a href="#about">About</a> ·
  <a href="#contributing">Contributing</a> ·
  <a href="#license">License</a>
</p>

---


> **Status: Alpha**  
> Tiles is currently alpha-quality software. It is usable for everyday tasks, though you may encounter bugs and performance issues. Tilekit, the developer SDK, is experimental, not a current priority, and intended for exploratory use, not production.

## Getting Started

There are two primary ways to work with Tiles, depending on whether you are an end user or a developer.

## Tiles CLI

Tiles is a local-first private AI assistant for everyday use.

Install the signed macOS package:

https://tiles.run/download

Then run the following command to start Tiles:

```bash
tiles
```

## Tilekit SDK

Tilekit is the SDK for developers to build on the infrastructure behind Tiles. It aims to be the app-server interface behind Tiles and future rich client experiences. Developers can embed it into their local clients by bundling or fetching a platform-specific App Server binary, running as a long-lived child process and communicating over bidirectional stdio JSON-RPC.

## Documentation

Full documentation is available in the Tiles Book:  
https://tiles.run/book

## About

Tiles is built by a small team working on private, local-first software, with a clear mission: to bring privacy technology to everyone. 

This project is part of the [User & Agents](https://userandagents.com) network. The shared goal is to empower people by designing and building software that provides agency, control, and choice in our digital lives. We strive to deliver the best privacy-focused engineering while also offering unmatched convenience in our consumer products. We believe identity and memory belong together, and Tiles gives you a way to own both through your personal user agent.

## Contributing

Ideas, issues, and pull requests are welcome.

Start here:
- [Contributing to Tiles](CONTRIBUTING.md)
- [Developing Tiles](HACKING.md)

## License

This project is dual-licensed under MIT and Apache 2.0:

- [MIT License](https://github.com/tilesprivacy/tiles/blob/main/LICENSE-MIT)
- [Apache License 2.0](https://github.com/tilesprivacy/tiles/blob/main/LICENSE-APACHE)

Downstream projects and end users may chose either license individually, or both together, at their discretion. The motivation for this dual-licensing is the additional software patent assurance provided by Apache 2.0.
