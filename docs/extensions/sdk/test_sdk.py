"""Disconnected byte-stream tests; no Desktop, network or native user state."""
import io
import json
import unittest
from nxtools import Session, HostError
RUN='11111111-1111-4111-8111-111111111111'
def frame(**fields):return (json.dumps({'protocol_version':1,'run_id':RUN,**fields})+'\n').encode()
class Tests(unittest.TestCase):
    def test_calls_pages_completion(self):
        source=io.BytesIO(frame(op='invoke')+frame(op='host_result',call_id='call-1',success=True,value={'items':[{'id':1}],'has_more':True,'next_cursor':'next'})+frame(op='host_result',call_id='call-2',success=True,value={'items':[{'id':2}],'has_more':False,'next_cursor':None}))
        output=io.BytesIO();session=Session(source,output)
        self.assertEqual([p['items'][0]['id'] for p in session.pages('downloads.list_page',limit=1)],[1,2]);session.complete('handled',{'text':'done'})
        frames=[json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(frames[1]['params']['cursor'],'next');self.assertEqual(frames[-1]['status'],'handled')
        with self.assertRaises(ValueError):session.complete()
    def test_error_never_auto_retries(self):
        output=io.BytesIO();session=Session(io.BytesIO(frame(op='invoke')+frame(op='host_result',call_id='call-1',success=False,value={'code':'native_failed','native_error_code':5,'private':'secret'})),output)
        with self.assertRaises(HostError) as caught:session.call('common.copy',{'text':'fixture'})
        self.assertEqual(caught.exception.code,'native_failed');self.assertEqual(str(caught.exception),'native_failed');self.assertEqual(len(output.getvalue().splitlines()),1)
    def test_rejects_invalid_identity_duplicate_and_size(self):
        for raw in [b'{"x":1,"x":2}\n',b'x'*262145+b'\n',frame(op='invoke',protocol_version=2)]:
            with self.assertRaises((ValueError,KeyError)):Session(io.BytesIO(raw),io.BytesIO())
    def test_v2_explicit_handshake_identity_and_request_finished(self):
        sid='22222222-2222-4222-8222-222222222222'
        source=io.BytesIO(frame(protocol_version=2,op='initialize',session_id=sid,mode='once',limits={})+frame(protocol_version=2,op='invoke',session_id=sid)+frame(protocol_version=2,op='host_result',session_id=sid,call_id='call-1',success=True,value={'ok':True}))
        output=io.BytesIO();session=Session(source,output,protocol_version=2)
        self.assertEqual(session.call('host.describe',{}),{'ok':True});session.complete()
        sent=[json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual([f['op'] for f in sent],['ready','host_call','complete','request_finished'])
        self.assertNotIn('run_id',sent[0]);self.assertTrue(all(f['session_id']==sid and f['protocol_version']==2 for f in sent))
    def test_v2_does_not_silently_negotiate_or_accept_other_session(self):
        sid='22222222-2222-4222-8222-222222222222'
        init=frame(protocol_version=2,op='initialize',session_id=sid,mode='once',limits={})
        with self.assertRaises(ValueError):Session(io.BytesIO(init),io.BytesIO())
        with self.assertRaises(ValueError):Session(io.BytesIO(init+frame(protocol_version=2,op='invoke',session_id=RUN)),io.BytesIO(),protocol_version=2)
        with self.assertRaises(ValueError):Session(io.BytesIO(frame(op='invoke')),io.BytesIO(),protocol_version=2)
    def test_v2_blob_exact_bytes_ack_digest_progress_and_no_v1_fallback(self):
        import base64,hashlib
        sid='22222222-2222-4222-8222-222222222222'
        blob='33333333-3333-4333-8333-333333333333'; upload='44444444-4444-4444-8444-444444444444'; download='55555555-5555-4555-8555-555555555555'
        payload=b'abc'; descriptor=dict(blob_id=blob,length=3,media_type='text/plain',sha256=hashlib.sha256(payload).hexdigest())
        def v2(**fields):return frame(protocol_version=2,session_id=sid,**fields)
        source=io.BytesIO(v2(op='initialize',mode='once',limits={})+v2(op='invoke')+
            v2(op='host_result',call_id='call-1',success=True,value=dict(descriptor,transfer_id=upload))+
            v2(op='data_ack',transfer_id=upload,sequence=1,offset=3)+
            v2(op='host_result',call_id='call-2',success=True,value=descriptor)+
            v2(op='host_result',call_id='call-3',success=True,value=dict(transfer_id=download,length=3))+
            v2(op='data',transfer_id=download,sequence=1,offset=0,data=base64.b64encode(payload).decode())+
            v2(op='host_result',call_id='call-4',success=True,value={}))
        output=io.BytesIO();session=Session(source,output,protocol_version=2)
        session.progress(1,2,'Working');session.progress(message='Unknown')
        sealed=session.blob_upload(payload,'text/plain');self.assertEqual(session.blob_read(sealed),payload)
        session.blob_release(sealed);session.complete()
        sent=[json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual([f['op'] for f in sent],['ready','progress','progress','host_call','data','host_call','host_call','data_ack','host_call','complete','request_finished'])
        self.assertEqual(session.calls,4)
        legacy=Session(io.BytesIO(frame(op='invoke')),io.BytesIO())
        with self.assertRaisesRegex(ValueError,'unsupported_transport'):legacy.blob_upload(payload)
        with self.assertRaisesRegex(ValueError,'unsupported_transport'):legacy.progress(1,2)
    def test_v2_progress_bounds_before_write(self):
        sid='22222222-2222-4222-8222-222222222222'
        session=Session(io.BytesIO(frame(protocol_version=2,op='initialize',session_id=sid,mode='once')+frame(protocol_version=2,op='invoke',session_id=sid)),io.BytesIO(),protocol_version=2)
        for values in [(1,None),(True,2),(1,0),(3,2),(0,2**53)]:
            with self.assertRaises(ValueError):session.progress(*values)
        with self.assertRaises(ValueError):session.progress(message='汉'*171)
        with self.assertRaises(ValueError):session.progress(message='\x1b')
        for _ in range(256):session.progress()
        with self.assertRaises(ValueError):session.progress()
    def test_storage_v2_null_cas_list_and_error_never_retries(self):
        sid='22222222-2222-4222-8222-222222222222'
        def v2(**fields):return frame(protocol_version=2,session_id=sid,**fields)
        source=io.BytesIO(v2(op='initialize',mode='once')+v2(op='invoke')+
            v2(op='host_result',call_id='call-1',success=True,value={'revision':1,'cleanup_pending':False})+
            v2(op='host_result',call_id='call-2',success=True,value={'found':True,'value':None,'revision':1})+
            v2(op='host_result',call_id='call-3',success=True,value={'items':[],'revision':1,'has_more':False,'next_cursor':None})+
            v2(op='host_result',call_id='call-4',success=False,value={'code':'storage_conflict'}))
        output=io.BytesIO();session=Session(source,output,protocol_version=2)
        self.assertEqual(session.storage_set('key',None,expected_revision=0)['revision'],1)
        self.assertIsNone(session.storage_get('key')['value'])
        self.assertFalse(session.storage_list(prefix='k',limit=1)['has_more'])
        with self.assertRaises(HostError):session.storage_delete('key',expected_revision=0)
        sent=[json.loads(line) for line in output.getvalue().splitlines() if json.loads(line)['op']=='host_call']
        self.assertEqual(len(sent),4);self.assertEqual(sent[0]['params'],{'key':'key','value':None,'expected_revision':0})
        legacy=Session(io.BytesIO(frame(op='invoke')),io.BytesIO())
        with self.assertRaisesRegex(ValueError,'unsupported_transport'):legacy.storage_get('key')
if __name__=='__main__':unittest.main()
