"""Source/docs parity gate; does not replace host/native/visual acceptance."""
import importlib.util
import json
from pathlib import Path
import re
import unittest
ROOT=Path(__file__).resolve().parent
WORKSPACE=ROOT.parents[1]
class Contracts(unittest.TestCase):
    def test_schema_is_reproducible_and_method_names_match_host(self):
        spec=importlib.util.spec_from_file_location('builder',ROOT/'build_schemas.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
        schemas=module.generate()
        for name,schema in schemas.items():self.assertEqual(json.loads((ROOT/name).read_text(encoding='utf-8')),schema)
        source=(WORKSPACE/'rust/crates/nxtools-extensions/src/api.rs').read_text(encoding='utf-8')
        names=set(re.findall(r'\("([a-z_]+\.[a-z_]+)",',source))
        definitions=schemas['methods.schema.json']['$defs']
        self.assertEqual(names,{name for name in definitions if '.' in name})
        def walk(value,root):
            if isinstance(value,dict):
                if '$ref' in value:
                    self.assertTrue(value['$ref'].startswith('#/$defs/'));self.assertIn(value['$ref'].split('/')[-1],root['$defs'])
                for item in value.values():walk(item,root)
            elif isinstance(value,list):
                for item in value:walk(item,root)
        for schema in schemas.values():walk(schema,schema)
    def test_network_origin_declaration_structure_matches_runtime_limits(self):
        manifest=json.loads((ROOT/'manifest.schema.json').read_text(encoding='utf-8'))
        network=manifest['properties']['network']
        self.assertEqual(network['type'],'object')
        self.assertFalse(network['additionalProperties'])
        self.assertEqual(network['required'],[])
        origins=network['properties']['origins']
        self.assertEqual(origins['type'],'array')
        self.assertEqual(origins['maxItems'],32)
        self.assertTrue(origins['uniqueItems'])
        self.assertEqual(origins['items']['type'],'string')
        self.assertEqual(origins['items']['minLength'],1)
        self.assertEqual(origins['items']['maxLength'],8192)
        source=(WORKSPACE/'rust/crates/nxtools-extensions/src/network_declaration.rs').read_text(encoding='utf-8')
        self.assertIn('MAX_NETWORK_ORIGINS: usize = 32;',source)
        self.assertIn('MAX_ORIGIN_BYTES: usize = 8192;',source)
        methods=json.loads((ROOT/'methods.schema.json').read_text(encoding='utf-8'))
        self.assertNotIn('http.request',methods['$defs'],'declarations alone do not enable HTTP')
    def test_examples_declare_only_supported_input_and_listed_files(self):
        for name in ['clipboard-text-tools','download-link-list','github-public-proxy']:
            package=ROOT/'examples'/name;manifest=json.loads((package/'extension.json').read_text(encoding='utf-8'))
            for path in manifest['files']:self.assertTrue((package/path).is_file())
            for handler in manifest['handlers']:
                accepted=handler.get('accepts')
                if accepted:
                    self.assertTrue(1<=accepted.get('min_items',1)<=accepted.get('max_items',32)<=32)
                    self.assertTrue(set(accepted['kinds'])<={'text','files','image','download'})
if __name__=='__main__':unittest.main()
