Shell command / UI action
        │
        ▼
App::execute_command(&str)
        │
        ├── filesystem commands (cd, ls)
        │       └── FsReader::read_dir(path) → ViewContent::Filesystem
        │
        └── library commands (add, remove, list_all, search)
                └── LibraryService → ViewContent::Tracklist
                        ├── reads index.redb for queries
                        └── reads meta/<id>.toml for full track data
