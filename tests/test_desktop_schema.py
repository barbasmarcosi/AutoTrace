"""Pruebas SQLite reales sin Rust para la migración v1 de AutoTrace Desktop."""
import pathlib
import sqlite3
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
SQL = (ROOT / 'desktop/src-tauri/migrations/001_init.sql').read_text(encoding='utf-8')


class DesktopSchemaTests(unittest.TestCase):
    def setUp(self):
        self.db = sqlite3.connect(':memory:')
        self.db.execute('PRAGMA foreign_keys=ON')
        self.db.executescript(SQL)

    def tearDown(self):
        self.db.close()

    def client(self, client_id='c1'):
        self.db.execute('INSERT INTO clients VALUES (?, ?, ?)', (client_id, 'Lucía', '{"id":"c1"}'))

    def vehicle(self, vehicle_id='v1', plate='AF123BC', client_id='c1'):
        self.db.execute('INSERT INTO vehicles VALUES (?, ?, ?, ?)',
                        (vehicle_id, client_id, plate, '{"id":"v1"}'))

    def test_initial_schema_and_revision(self):
        self.assertEqual(self.db.execute('PRAGMA user_version').fetchone()[0], 1)
        self.assertEqual(self.db.execute("SELECT int_value FROM app_meta WHERE key='revision'").fetchone()[0], 0)
        self.assertEqual(self.db.execute('PRAGMA foreign_key_check').fetchall(), [])

    def test_plate_unique_and_vehicle_requires_client(self):
        with self.assertRaises(sqlite3.IntegrityError):
            self.vehicle()
        self.client()
        self.vehicle()
        with self.assertRaises(sqlite3.IntegrityError):
            self.vehicle('v2', 'AF123BC')
        with self.assertRaises(sqlite3.IntegrityError):
            self.vehicle('v2', 'af123bc')

    def test_orders_have_referential_integrity_and_money_checks(self):
        self.client()
        self.vehicle()
        valid=('o1', 'v1', 'progress', 1100, 500, 200, 300, '{}')
        self.db.execute('INSERT INTO orders VALUES (?,?,?,?,?,?,?,?)', valid)
        for changes in [(2, 'not-valid'), (3, -1), (4, 1200), (5, -1), (6, -1)]:
            row = list(valid)
            row[changes[0]] = changes[1]
            row[0] = 'invalid'+str(changes[0])
            with self.assertRaises(sqlite3.IntegrityError):
                self.db.execute('INSERT INTO orders VALUES (?,?,?,?,?,?,?,?)', row)
        with self.assertRaises(sqlite3.IntegrityError):
            self.db.execute('DELETE FROM vehicles WHERE id=?', ('v1',))

    def test_invalid_record_json_rejected(self):
        with self.assertRaises(sqlite3.IntegrityError):
            self.db.execute('INSERT INTO clients VALUES (?,?,?)', ('bad', 'Pepe', '{invalid json'))

    def test_batch_rollback_preserves_original_data(self):
        self.client()
        self.vehicle()
        self.db.commit()
        with self.assertRaises(sqlite3.IntegrityError):
            with self.db:
                self.db.execute('DELETE FROM vehicles')
                self.db.execute('DELETE FROM clients')
                self.db.execute('INSERT INTO vehicles VALUES (?,?,?,?)', ('orphan','absent','AB123CD','{}'))
        self.assertEqual(self.db.execute('SELECT count(*) FROM clients').fetchone()[0], 1)
        self.assertEqual(self.db.execute('SELECT count(*) FROM vehicles').fetchone()[0], 1)


if __name__ == '__main__':
    unittest.main()
