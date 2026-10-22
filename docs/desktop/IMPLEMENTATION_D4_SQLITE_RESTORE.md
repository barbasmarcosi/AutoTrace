# SQLite recovery — D4 partial implementation

Desktop Ajustes includes **Recuperar SQLite**. The native command opens a selected file read-only, checks `user_version`, `PRAGMA integrity_check`, foreign-key consistency and the AutoTrace data schema, and optionally verifies the corresponding manifest SHA-256 + snapshot identity. The UI displays counts and asks for explicit replacement confirmation; it creates a fresh dual backup before saving through the existing revision/CAS and SQL transaction, without copying the untrusted SQLite file into the live location.

**Not production-ready:** no restore-on-a-clean-Windows-PC acceptance yet, the selected file could change between preview and save, and no encrypted off-device backup exists. This pull request stays Draft.
