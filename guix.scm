; SPDX-License-Identifier: AGPL-3.0-or-later
;; GNU Guix development environment for IDApTIK.
;; Usage: guix shell -f guix.scm

(use-modules (guix packages)
             (guix build-system gnu)
             (guix licenses)
             (gnu packages build-tools)
             (gnu packages elixir)
             (gnu packages erlang)
             (gnu packages rust)
             (gnu packages zig))

(package
  (name "idaptik-env")
  (version "0.1.0")
  (source #f)
  (build-system gnu-build-system)
  (native-inputs
   (list rust zig just elixir erlang))
  (synopsis "Development environment for IDApTIK")
  (description "Provides the core Rust, Zig, Elixir, Erlang, and task-runner toolchains used by IDApTIK.")
  (home-page "https://github.com/metadatastician/IDApTIK")
  (license agpl3+))
