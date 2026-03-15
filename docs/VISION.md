This document outlines the vision of the project.

The library of Alexandria took a copy of every book that passed through the port. The aim of this project is to do the same, but for every page a user reads online.
The goal is to provide a secure personal searchable index of their browser history. It comprises two parts:
1. A browser extension that captures raw page HTML
2. A native front-end application (Swift) that communicates to a Rust back-end via FFI that accomplishes indexing/search functionality.


The backend should take care of receiving snapshots and indexing them. It should have a pluggable mechanism for different ways to deliver page content. It should also provide a search interface.
The indexing backend should allow us to start/stop indexing behavior based on system power state (e.g. when in Low Power mode, don't process the indexing queue).
