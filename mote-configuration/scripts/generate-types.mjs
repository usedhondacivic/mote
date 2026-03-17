// Generates mote_api_types.ts from the JSON schemas produced by mote-ffi's build script.
import { compile } from 'json-schema-to-typescript';
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const schemasDir = resolve(__dirname, '../../mote-ffi/schemas');

const options = {
    bannerComment: '',
    additionalProperties: false,
    enableConstEnums: false,
};

const hostToMoteSchema = { ...JSON.parse(readFileSync(`${schemasDir}/host_to_mote.json`, 'utf8')), title: 'HostToMoteMessage' };
const moteToHostSchema = { ...JSON.parse(readFileSync(`${schemasDir}/mote_to_host.json`, 'utf8')), title: 'MoteToHostMessage' };

const hostTypes = await compile(hostToMoteSchema, 'HostToMoteMessage', options);
const moteTypes = await compile(moteToHostSchema, 'MoteToHostMessage', options);

const banner = '// Generated from mote-api message schemas — do not edit.\n\n';
const content = banner + hostTypes + '\n' + moteTypes;

const out = resolve(__dirname, '../src/lib/mote_api_types.ts');
writeFileSync(out, content);
console.log(`Generated ${out}`);
