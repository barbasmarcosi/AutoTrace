#!/usr/bin/env python3
"""Synthetic persisted SQLite v1 upgrade/downgrade integrity fixture.

Never touches real workshop databases: caller must give an explicit path in an
ephemeral Actions runner. Keeps expected checksums outside AppData.
"""
import argparse
import hashlib
import json
import sqlite3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCHEMA = ROOT / "desktop/src-tauri/migrations/001_init.sql"

def canonical(item):
    return json.dumps(item, ensure_ascii=False, sort_keys=True, separators=(",", ":"))

CLIENT = {
    "id":"historical-c-1","name":"Cliente sintético legado","phone":"0000",
    "email":"","notes":"TEST ONLY","createdAt":"2026-10-20"
}
VEHICLE = {
    "id":"historical-v-1","clientId":"historical-c-1","plate":"AF123BC",
    "make":"MarcaPrueba","model":"ModeloPrueba","year":2020,
    "mileage":12345.0,"vin":"","color":"","nextServiceDate":"",
    "notes":"","createdAt":"2026-10-20"
}
ORDER = {
    "id":"historical-o-1","vehicleId":"historical-v-1","title":"Trabajo ficticio",
    "category":"Diagnostico","status":"done","openedAt":"2026-10-20",
    "completedAt":"2026-10-21","mileage":12345.0,"description":"",
    "partsCost":15.0,"laborCost":40.0,"charged":90.99,"paid":23.45,
    "notes":"TEST ONLY","createdAt":"2026-10-20",
    "payments":[{"id":"pay-v1","date":"2026-10-21","amount":23.45,"method":"transfer",
                 "accountId":"acct-v1","reference":"TEST ONLY"}]
}
SETTINGS = {
    "garageName":"Taller QA ficticio","currency":"ARS","demo":False,
    "accounts":[{"id":"acct-v1","name":"Cuenta de prueba","kind":"bank","alias":"TEST ONLY"}]
}

def sha256(path):
    h = hashlib.sha256()
    with open(path,"rb") as f:
        for buf in iter(lambda:f.read(1024*1024),b""):
            h.update(buf)
    return h.hexdigest()

def ensure_valid(conn):
    if conn.execute("PRAGMA integrity_check").fetchone() != ("ok",):
        raise AssertionError("SQLite integrity check failed")
    if conn.execute("PRAGMA foreign_key_check").fetchall():
        raise AssertionError("foreign key check failed")
    if conn.execute("PRAGMA user_version").fetchone() != (1,):
        raise AssertionError("historical SQLite version changed")
    if conn.execute("SELECT int_value FROM app_meta WHERE key='revision'").fetchone() != (7,):
        raise AssertionError("historical revision was modified")
    records = (
        ("clients", CLIENT),
        ("vehicles", VEHICLE),
        ("orders", ORDER),
        ("app_settings", SETTINGS),
    )
    for table, expected in records:
        rows=conn.execute(f"SELECT record_json FROM {table}").fetchall()
        if len(rows)!=1 or json.loads(rows[0][0]) != expected:
            raise AssertionError(f"historical {table} data changed")
    row=conn.execute("SELECT charged_cents,paid_cents,parts_cents,labor_cents FROM orders").fetchone()
    if row!=(9099,2345,1500,4000):
        raise AssertionError("historical monetary balances changed")

def prepare(database, evidence):
    if database.exists():
        raise AssertionError("refusing to overwrite any pre-existing SQLite file")
    if evidence.exists():
        raise AssertionError("expected SHA manifest already exists")
    database.parent.mkdir(parents=True,exist_ok=True)
    conn=sqlite3.connect(database)
    try:
        conn.execute("PRAGMA foreign_keys=ON")
        conn.executescript(SCHEMA.read_text(encoding="utf-8"))
        conn.execute("INSERT INTO clients VALUES(?,?,?)",(CLIENT["id"],CLIENT["name"],canonical(CLIENT)))
        conn.execute("INSERT INTO vehicles VALUES(?,?,?,?)",
                     (VEHICLE["id"],CLIENT["id"],VEHICLE["plate"],canonical(VEHICLE)))
        conn.execute("INSERT INTO orders VALUES(?,?,?,?,?,?,?,?)",
                     (ORDER["id"],VEHICLE["id"],ORDER["status"],9099,2345,1500,4000,canonical(ORDER)))
        conn.execute("INSERT INTO app_settings VALUES(1,?)",(canonical(SETTINGS),))
        conn.execute("UPDATE app_meta SET int_value=7 WHERE key='revision'")
        conn.commit()
        ensure_valid(conn)
    finally:
        conn.close()
    evidence.write_text(sha256(database)+"\n",encoding="ascii")
    print(f"PREPARED synthetic v1: {database.name}, sha256={sha256(database)}")

def verify(database,evidence):
    expected=evidence.read_text(encoding="ascii").strip()
    actual=sha256(database)
    if expected!=actual:
        raise AssertionError("installer upgrade changed live SQLite bytes")
    conn=sqlite3.connect(f"file:{database.as_posix()}?mode=ro",uri=True)
    try:
        conn.execute("PRAGMA foreign_keys=ON")
        ensure_valid(conn)
    finally:
        conn.close()
    print(f"PASS historical SQLite unchanged, sha256={actual}")

if __name__=="__main__":
    p=argparse.ArgumentParser()
    p.add_argument("command",choices=["prepare","verify"])
    p.add_argument("database",type=Path)
    p.add_argument("expected_sha_file",type=Path)
    args=p.parse_args()
    if args.command=="prepare":
        prepare(args.database,args.expected_sha_file)
    else:
        verify(args.database,args.expected_sha_file)
