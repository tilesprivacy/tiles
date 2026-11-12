## How the Tiles pkgs are build

### Network Installer

Network installer is basically Tiles without any ML models included in it.
So when model is needed, Tiles will download it. (Maybe in a later version
a user should be able to download from its peers locally too).

```
just bundle_pkg
```

Creates pkg/tiles-VERSION-signed.pkg, like tiles-0.5.0-signed.pkg, signs and notarize it


### Offline Installer

Offline Installer includes the default model too in it, so once
downloaded provides a portable installer, and can work w/o
internet forever and ever...

```
just bundle_model_pkg

```

This will bundle only the model in a .pkg.

> We run this command only when a model is updated/added etc..
Since this is a time taking process and is not needed to run
in every release build

The basic approach we will take for offline installer building is that
we build 2 pkgs essentially, the network installer and a pkg with
only models. Then we create a final package that has these 2 pkgs with
the command below.


```
just bundle_pkg_full

```
Creates tiles-full.pkg, signs and notarize it
