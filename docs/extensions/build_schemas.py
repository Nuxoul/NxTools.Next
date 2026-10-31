"""Reproducible public authoring schemas; runtime validation remains authoritative."""
import json
from pathlib import Path
ROOT = Path(__file__).resolve().parent

def obj(properties, required=(), **extra):
    return dict(type='object', properties=properties, required=list(required), additionalProperties=False, **extra)
def ref(name): return {'$ref': '#/$defs/' + name}
def array(item, maximum): return dict(type='array', items=item, maxItems=maximum)
def text(maximum=32768): return dict(type='string', maxLength=maximum)
ID = dict(type='string', pattern=r'^[a-z0-9._-]+$', maxLength=64, minLength=1)
UUID = dict(type='string', format='uuid')
NULL_UUID = {'anyOf': [UUID, {'type': 'null'}]}
LIMIT = dict(type='integer', minimum=1, maximum=32, default=32)
STATUS = {'enum': ['created','probing','waiting','downloading','paused','finalizing','verifying','completed','failed','cancelled']}
FEATURE_ENTRY = [('quick_launch','command'),('quick_launch','provider'),('tools','action'),('downloads','resolve'),('downloads','action'),('downloads','created'),('downloads','completed'),('downloads','failed'),('clipboard','action'),('clipboard','committed')]

def generate():
    action = {'oneOf': [
        obj({'type':{'const':'website'},'url_template':text()}, ['type','url_template']),
        obj({'type':{'const':'folder'},'path':text()}, ['type','path']),
        obj({'type':{'const':'program'},'executable':text(),'argument_template':text(),'working_directory':text()}, ['type','executable']),
        obj({'type':{'const':'script'},'entry_file':text(1024)}, ['type','entry_file']),
        obj({'type':{'const':'items'},'items':array(obj({'id':ID,'title':text(256),'action':ref('action')},['id','title','action']),200)}, ['type','items'])]}
    setting_base={'key':ID,'title':text(256),'description':text(4096)}
    validation=obj({'required':{'type':'boolean','default':False},'min_length':{'type':'integer','minimum':0,'maximum':32768},'max_length':{'type':'integer','minimum':0,'maximum':32768},'pattern':text(512)})
    choice_options={'oneOf':[array(text(256),100),array(obj({'value':text(256),'label':text(256)},['value','label']),100)]}
    settings=[]
    for kind, fields, required in [
        ('text',{'default':text(),'validation':validation},[]),('bool',{'default':{'type':'boolean'}},[]),
        ('number',{'default':{'type':'integer'},'min':{'type':'integer'},'max':{'type':'integer'}},['default','min','max']),
        ('choice',{'default':text(256),'options':choice_options},['default','options']),
        ('file',{'default':text(),'validation':validation},[]),('folder',{'default':text(),'validation':validation},[]),('password',{},[])]:
        settings.append(obj(dict(setting_base,type={'const':kind},**fields),['key','title','type',*required]))
    handler=obj({'id':ID,'title':text(256),'feature':text(32),'entry':text(32),'keywords':array(text(64),32),'action':ref('action'),
                 'url_matches':array(obj({'host':text(253),'include_subdomains':{'type':'boolean'},'path':text(2048),'priority':{'type':'integer'}},['host']),32),
                 'accepts':obj({'kinds':array({'enum':['text','files','image','download']},4),'min_items':{'type':'integer','minimum':1,'maximum':32},'max_items':{'type':'integer','minimum':1,'maximum':32},'max_bytes':{'type':'integer','minimum':1,'maximum':131072}})},['id','title','feature','entry','action'])
    handler['allOf']=[{'anyOf':[{'properties':{'feature':{'const':feature},'entry':{'const':entry}}} for feature,entry in FEATURE_ENTRY]}]
    manifest=obj({'schema_version':{'const':1},'id':ID,'name':text(256),'version':text(64),'description':text(4096),'author':text(256),
        'permissions':array(obj({'id':ID,'reason':text(1024)},['id','reason']),64),
        'handlers':dict(array(ref('handler'),100),minItems=1),
        'runtime':obj({'program':{'enum':['python','node','powershell','pwsh','exe']},'arguments':array(text(),128),'timeout_ms':{'type':['integer','null'],'minimum':1,'maximum':120000},'protocol_version':{'enum':[1,2],'default':1},'mode':{'enum':['once','resident'],'default':'once'},'resident_entry_file':{'anyOf':[text(1024),{'type':'null'}]},'idle_exit_seconds':{'type':['integer','null'],'minimum':30,'maximum':600}},['program']),
        'files':array(text(1024),4096),'settings':array({'oneOf':settings},100),
        'requires':array(obj({'method':text(128),'version':{'const':1}},['method','version']),32),
        'network':obj({'origins':dict(array(dict(text(8192),minLength=1),32),uniqueItems=True)},
            **{'$comment':'Optional exact HTTP(S) origins. ImportPreview enforces UTF8 bytes, IDNA host/effective-port normalization, normalized uniqueness, and no credentials/wildcards/subnets/path/query/fragment. Empty declarations serialize away; declaring origins alone does not enable an HTTP method.'})},['schema_version','id','name','version','handlers'])
    manifest.update({'$schema':'https://json-schema.org/draft/2020-12/schema','$id':'https://nxtools.local/extensions/manifest.schema.json','$defs':{'action':action,'handler':handler},
        '$comment':'Authoring structure only. Exact UTF8 bytes/Unicode scalar lengths, bounded Rust full-string regex syntax, homogeneous/unique labeled choices, required-unconfigured import versus save/readiness, paths, unique IDs, known methods, runtime/code/trust and native validation remain authoritative in ImportPreview and host owners.'})
    request=obj({'source_url':text(),'save_directory':text(),'save_directory_explicit':{'type':'boolean'},'file_name':{'type':['string','null']},'headers':array(obj({'name':text(),'value':text()},['name','value']),128),'cookie':{'type':['string','null']},'referrer':{'type':['string','null']},'certificate_policy':{'enum':['strict','accept_invalid']},'connection_limit':{'type':['integer','null'],'minimum':1,'maximum':64},'speed_limit_bytes_per_second':{'type':['integer','null'],'minimum':1},'expected_total_bytes':{'type':['integer','null'],'minimum':0},'scheduled_at_unix_ms':{'type':['integer','null']},'adaptive_segment_policy':{'type':['object','null']},'media':{'type':['object','null']},'category_id':{'type':['string','null']},'category_name':{'type':['string','null']},'duplicate_strategy':{'type':'string'}},['source_url','save_directory'])
    request['additionalProperties']=True
    request['$comment']='Nested media/adaptive/category/duplicate semantics follow DownloadRequest validation; this schema is not an authorization bypass.'
    copy={'oneOf':[obj({'text':text(131072)},['text']),obj({'paths':array(text(4096),64)},['paths'])]}
    page=obj({'items':array({'type':'object'},32),'has_more':{'type':'boolean'},'next_cursor':NULL_UUID,'cursor_ttl_ms':{'const':60000},'consistency':{'type':'string'},'order':{'type':'string'}},['items','has_more','next_cursor','cursor_ttl_ms','consistency'])
    definitions={'request':request,'page':page,'error':obj({'code':text(128),'message':text(4096),'retryable':{'type':'boolean'},'native_error_code':{'type':['integer','null']}},['code'])}
    params={
        'storage.get':obj({'key':dict(text(128),minLength=1)},['key']),
        'storage.set':obj({'key':dict(text(128),minLength=1),'value':{},'expected_revision':{'type':['integer','null'],'minimum':0,'maximum':9007199254740991}},['key','value']),
        'storage.delete':obj({'key':dict(text(128),minLength=1),'expected_revision':{'type':['integer','null'],'minimum':0,'maximum':9007199254740991}},['key']),
        'storage.list':obj({'prefix':text(128),'limit':LIMIT,'cursor':NULL_UUID}),
        'blobs.begin':obj({'length':{'type':'integer','minimum':0,'maximum':8388608},'media_type':text(128),'sha256':{'type':'string','pattern':'^[a-f0-9]{64}$'}},['length','media_type','sha256']),
        'blobs.seal':obj({'blob_id':UUID},['blob_id']),
        'blobs.read':obj({'blob_id':UUID,'offset':{'type':'integer','minimum':0,'maximum':8388608},'length':{'type':'integer','minimum':0,'maximum':8388608}},['blob_id']),
        'blobs.release':obj({'blob_id':UUID},['blob_id']),
        'host.describe':obj({}),'credentials.get':obj({'key':ID},['key']),
        'downloads.list':obj({'limit':LIMIT}),'downloads.list_page':obj({'limit':LIMIT,'cursor':NULL_UUID,'statuses':array(STATUS,16)}),
        'downloads.get':obj({'task_id':UUID},['task_id']),
        'clipboard.get':obj({'item_id':{'type':'integer','minimum':1}},['item_id']),
        'clipboard.query':obj({'query':text(4096),'limit':LIMIT},['query']),
        'clipboard.query_page':obj({'query':text(4096),'limit':LIMIT,'cursor':NULL_UUID},['query']),
        'clipboard.create_derived':{'oneOf':[obj({'source_item_id':{'type':'integer','minimum':1},'text':text(131072)},['source_item_id','text']),obj({'source_item_id':{'type':'integer','minimum':1},'paths':array(text(4096),64)},['source_item_id','paths'])]},
        'clipboard.copy':copy,'common.copy':copy,'common.open':obj({'target':text()},['target']),
        'common.program':obj({'executable':text(),'arguments':array(text(),128),'working_directory':text()},['executable','arguments']),
        'common.notify':obj({'title':text(256),'message':text(4096)},['title','message']),
        'quick_launch.query':obj({'query':text(4096)},['query']),
        'quick_launch.search':obj({'query':text(4096)},['query']),
        'quick_launch.search_catalog':obj({'query':text(4096)},['query'])}
    for name in ['downloads.pause','downloads.resume','downloads.cancel']:params[name]=obj({'task_id':UUID},['task_id'])
    resolution=obj({'operation_id':UUID,'mode':{'oneOf':[obj({'kind':{'const':'resolve'}},['kind']),obj({'kind':{'const':'receipt'},'receipt':UUID},['kind','receipt'])]}})
    for name in ['downloads.probe','downloads.submit','downloads.enqueue']:params[name]=obj({'request':ref('request'),'idempotency_key':NULL_UUID,'resolution':resolution},['request'])
    for method, value in params.items():
        result=ref('page') if method.endswith('_page') else {'type':'array','items':{'type':'object'}} if method in ('downloads.list','clipboard.query','quick_launch.search') else {'type':'object'}
        revision={'type':'integer','minimum':0,'maximum':9007199254740991}
        if method=='storage.get':result=obj({'found':{'type':'boolean'},'value':{},'revision':revision},['found','value','revision'])
        elif method in ('storage.set','storage.delete'):result=obj({'revision':revision,'cleanup_pending':{'type':'boolean'}},['revision','cleanup_pending'])
        elif method=='storage.list':result=obj({'items':array(obj({'key':text(128),'revision':revision},['key','revision']),32),'revision':revision,'has_more':{'type':'boolean'},'next_cursor':NULL_UUID},['items','revision','has_more','next_cursor'])
        definitions[method]=obj({'params':value,'result':result},['params','result'])
    methods={'$schema':'https://json-schema.org/draft/2020-12/schema','$id':'https://nxtools.local/extensions/methods.schema.json','$defs':definitions,'$comment':'Each method definition validates a params/result pair. Static schemas are authoring aids, not replacement for feature-owner validation. UTF-8 byte budgets can be stricter than maxLength.'}
    return {'manifest.schema.json':manifest,'methods.schema.json':methods}
if __name__=='__main__':
    for name,schema in generate().items():
        (ROOT/name).write_text(json.dumps(schema,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print('Generated extension authoring schemas (no plugin code executed)')
