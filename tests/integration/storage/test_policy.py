"""Shared Rust SQL policy regression using an explicitly unencrypted SQLite double.
No encryption, native key protection or platform result is claimed by this test.
Generate Python UniFFI bindings beside the host library in .work/storage-python first.
"""
from pathlib import Path
import sqlite3,sys,unittest
ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'.work/storage-python'))
import meshchat_core as m

class Database:
    def __init__(self):
        self.db=sqlite3.connect(':memory:',isolation_level=None)
        self.fail=None
    def execute(self,sql,values):
        if self.fail and self.fail in sql:raise m.StorageError.Database()
        try:
            cursor=self.db.execute(sql,[x.value for x in values])
            # Match native execute: a row-returning statement needs query.
            if cursor.description is not None:raise m.StorageError.Database()
        except sqlite3.Error:raise m.StorageError.Database() from None
    def query(self,sql,values,limit):
        # This is a test double, not a claim that ordinary SQLite is encrypted.
        if sql=='PRAGMA cipher_version':return [m.SqlRow(cells=[m.SqlValue.TEXT('test-double')])]
        try:rows=self.db.execute(sql,[x.value for x in values]).fetchmany(limit+1)
        except sqlite3.Error:raise m.StorageError.Database() from None
        def value(x):return m.SqlValue.INTEGER(x) if isinstance(x,int) else m.SqlValue.BYTES(x) if isinstance(x,bytes) else m.SqlValue.TEXT(x)
        return [m.SqlRow(cells=[value(x) for x in row]) for row in rows]

GEN=b'1'*16;CHANNEL=b'chan';PEER=b'p'*64;SUBJECT=b'\x02'+PEER;NOW=200000

def item(direct=False,id=1,stamp=NOW):
    return m.HistoryItem(conversation=PEER if direct else CHANNEL,direct=direct,direction=0,logical_type=1,message_id=id.to_bytes(8,'big'),timestamp=stamp,body=b'synthetic encrypted history marker',provenance=b'verified fixture' if direct else b'')

class Policy(unittest.TestCase):
    def setUp(self):self.d=Database();self.s=m.EncryptedStore.open(self.d,GEN,True,NOW)
    def test_schema_restart_and_records(self):
        for kind,key in [(m.RecordKind.SETTING,b'theme'),(m.RecordKind.SUBSCRIPTION,CHANNEL),(m.RecordKind.FRIEND,PEER),(m.RecordKind.EVENT_ROOT,b'r'*32),(m.RecordKind.STAFF_CREDENTIAL,b's'*64)]:
            self.s.put_record(kind,key,b'opaque canonical fixture');self.assertEqual(self.s.get_record(kind,key),b'opaque canonical fixture')
        self.s.append_unverified(item(),NOW)
        reopened=m.EncryptedStore.open(self.d,GEN,False,NOW)
        self.assertEqual(reopened.history(CHANNEL,False,100)[0].body,item().body)
        with self.assertRaises(m.StorageError.Schema):m.EncryptedStore.open(self.d,b'2'*16,False,NOW)
        with self.assertRaises(m.StorageError.Schema):m.EncryptedStore.open(self.d,GEN,True,NOW)
    def test_atomic_replay_conflict_and_history_deletion(self):
        msg=item(True)
        self.assertEqual(self.s.accept_authenticated(msg,SUBJECT,b'immutable',NOW),m.AcceptResult.ACCEPTED)
        self.s.delete_history(PEER,True)
        reopened=m.EncryptedStore.open(self.d,GEN,False,NOW)
        self.assertEqual(reopened.accept_authenticated(msg,SUBJECT,b'immutable',NOW),m.AcceptResult.REPLAY)
        self.assertEqual(reopened.accept_authenticated(msg,SUBJECT,b'other',NOW),m.AcceptResult.CONFLICT)
        self.assertTrue(reopened.ambiguous_target(SUBJECT,msg.message_id))
        self.assertEqual(reopened.history(PEER,True,100),[])
    def test_reaction_state_and_cross_type_target_ambiguity(self):
        chat=item(True)
        self.s.accept_authenticated(chat,SUBJECT,b'chat',NOW)
        reaction=item(True);reaction.logical_type=6;reaction.body=b'12345678'+bytes([1,2])
        self.assertEqual(self.s.accept_authenticated(reaction,SUBJECT,b'reaction',NOW),m.AcceptResult.ACCEPTED)
        self.assertTrue(self.s.ambiguous_target(SUBJECT,chat.message_id))
        self.assertEqual(len(self.s.history(PEER,True,100)),2)
    def test_public_legacy_direction_replay_and_ambiguous_records(self):
        import hashlib
        for prefix,size in [(1,32),(3,64)]:
            for ambiguous in [False,True]:
                with self.subTest(prefix=prefix,ambiguous=ambiguous):
                    db=Database();store=m.EncryptedStore.open(db,GEN,True,NOW)
                    subject=bytes([prefix])+b'k'*size
                    msg=item();msg.provenance=subject;msg.direction=1
                    self.assertEqual(store.accept_authenticated(msg,subject,b'original',NOW),m.AcceptResult.ACCEPTED)
                    self.assertEqual(db.db.execute('SELECT direction FROM ledger').fetchone()[0],0)
                    self.assertEqual(store.history(CHANNEL,False,10)[0].direction,1)
                    db.db.execute('UPDATE ledger SET direction=1')
                    second=hashlib.sha256(b'other' if ambiguous else b'original').digest()
                    db.db.execute('INSERT INTO ledger SELECT subject,0,logical_type,message_id,?,timestamp,conflict FROM ledger',(second,))
                    store.delete_history(CHANNEL,False)
                    store=m.EncryptedStore.open(db,GEN,False,NOW)
                    msg.direction=0
                    if ambiguous:
                        db.fail='UPDATE ledger SET conflict'
                        with self.assertRaises(m.StorageError.Database):store.accept_authenticated(msg,subject,b'original',NOW)
                        self.assertEqual(db.db.execute('SELECT sum(conflict) FROM ledger').fetchone()[0],0)
                        db.fail=None
                    expected=m.AcceptResult.CONFLICT if ambiguous else m.AcceptResult.REPLAY
                    self.assertEqual(store.accept_authenticated(msg,subject,b'original',NOW),expected)
                    self.assertEqual(db.db.execute('SELECT count(*),sum(conflict) FROM ledger').fetchone(),(2,2 if ambiguous else 0))
                    self.assertEqual(store.history(CHANNEL,False,10),[])
                    self.assertEqual(db.db.execute('PRAGMA user_version').fetchone()[0],2)
    def test_dm_replay_identity_keeps_direction_separation(self):
        msg=item(True)
        self.assertEqual(self.s.accept_authenticated(msg,SUBJECT,b'incoming',NOW),m.AcceptResult.ACCEPTED)
        msg.direction=1
        self.assertEqual(self.s.accept_authenticated(msg,SUBJECT,b'outgoing',NOW),m.AcceptResult.ACCEPTED)
        self.assertEqual(self.s.accept_authenticated(msg,SUBJECT,b'outgoing',NOW),m.AcceptResult.REPLAY)
        self.assertEqual(len(self.s.history(PEER,True,10)),2)
    def test_effect_failure_rolls_back_ledger(self):
        self.d.fail='INSERT INTO history'
        with self.assertRaises(m.StorageError.Database):self.s.accept_authenticated(item(True),SUBJECT,b'immutable',NOW)
        self.assertEqual(self.d.db.execute('SELECT count(*) FROM ledger').fetchone()[0],0)
        self.d.fail=None
        self.assertEqual(self.s.accept_authenticated(item(True),SUBJECT,b'immutable',NOW),m.AcceptResult.ACCEPTED)
    def test_migration_rollback_preserves_v1_and_data(self):
        self.s.put_record(m.RecordKind.SETTING,b'theme',b'original')
        self.d.db.executescript('DROP TABLE ledger;DROP TABLE clock;PRAGMA user_version=1;')
        for point in ['CREATE TABLE ledger','CREATE INDEX ledger_expiry','CREATE TABLE clock','INSERT INTO clock','PRAGMA user_version=2']:
            self.d.fail=point
            with self.assertRaises(m.StorageError.Database):m.EncryptedStore.open(self.d,GEN,False,NOW)
            self.assertEqual(self.d.db.execute('PRAGMA user_version').fetchone()[0],1)
            self.assertEqual(self.d.db.execute('SELECT value FROM records').fetchone()[0],b'original')
            self.assertEqual(self.d.db.execute("SELECT count(*) FROM sqlite_master WHERE name='ledger'").fetchone()[0],0)
        self.d.fail=None;self.s=m.EncryptedStore.open(self.d,GEN,False,NOW)
        self.assertEqual(self.s.get_record(m.RecordKind.SETTING,b'theme'),b'original')
    def test_clock_uncertainty_persists_and_requires_recovery(self):
        self.s.prune(NOW+1000)
        with self.assertRaises(m.StorageError.ClockUncertain):self.s.prune(NOW)
        self.s=m.EncryptedStore.open(self.d,GEN,False,NOW+800)
        with self.assertRaises(m.StorageError.ClockUncertain):self.s.prune(NOW+800)
        self.s.prune(NOW+1000)
        with self.assertRaises(m.StorageError.ClockUncertain):self.s.prune(None)
        self.s.prune(NOW+1000)
    def test_pruning_and_live_tombstone_boundary(self):
        self.s.accept_authenticated(item(True),SUBJECT,b'original',NOW)
        self.s.prune(NOW+172800)
        self.assertEqual(self.d.db.execute('SELECT count(*) FROM ledger').fetchone()[0],1)
        self.s.prune(NOW+172801)
        self.assertEqual(self.d.db.execute('SELECT count(*) FROM ledger').fetchone()[0],0)
        self.assertEqual(self.s.history(PEER,True,100),[])
    def test_per_channel_and_conversation_caps(self):
        sql='INSERT INTO history(conversation,direct,direction,logical_type,message_id,timestamp,body,provenance) VALUES(?,?,0,1,?,?,?,?)'
        for direct,conversation,cap in [(False,CHANNEL,5000),(True,PEER,1000)]:
            self.d.db.executemany(sql,[(conversation,int(direct),i.to_bytes(8,'big'),NOW,b'x',b'') for i in range(cap)])
            if direct:self.s.accept_authenticated(item(True,99999),SUBJECT,b'new',NOW)
            else:self.s.append_unverified(item(False,99999),NOW)
            self.assertEqual(self.d.db.execute('SELECT count(*) FROM history WHERE direct=?',(int(direct),)).fetchone()[0],cap)
    def test_ledger_full_refuses_without_eviction(self):
        self.d.db.executemany('INSERT INTO ledger(subject,direction,logical_type,message_id,digest,timestamp) VALUES(?,0,1,?,?,?)',[(SUBJECT,i.to_bytes(8,'big'),b'd'*32,NOW) for i in range(100000)])
        with self.assertRaises(m.StorageError.Capacity):self.s.accept_authenticated(item(True,100001),SUBJECT,b'new',NOW)
        self.assertEqual(self.d.db.execute('SELECT count(*) FROM ledger').fetchone()[0],100000)
        self.assertEqual(self.s.history(PEER,True,100),[])
    def test_unverified_cannot_insert_dm_or_provenance(self):
        with self.assertRaises(m.StorageError.InvalidInput):self.s.append_unverified(item(True),NOW)
        msg=item();msg.provenance=b'unearned'
        with self.assertRaises(m.StorageError.InvalidInput):self.s.append_unverified(msg,NOW)
        with self.assertRaises(m.StorageError.InvalidInput):self.s.accept_authenticated(item(True),b'\x02'+b'q'*64,b'valid',NOW)
    def test_record_cap_is_transactional(self):
        for i in range(128):self.s.put_record(m.RecordKind.FRIEND,i.to_bytes(64,'big'),b'pin')
        with self.assertRaises(m.StorageError.Capacity):self.s.put_record(m.RecordKind.FRIEND,b'z'*64,b'pin')
        self.assertIsNone(self.s.get_record(m.RecordKind.FRIEND,b'z'*64))
        self.s.delete_record(m.RecordKind.FRIEND,(1).to_bytes(64,'big'))
        self.s.put_record(m.RecordKind.FRIEND,b'z'*64,b'pin')

if __name__=='__main__':unittest.main()
