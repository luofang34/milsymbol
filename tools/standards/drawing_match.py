from drawing_config import WORK, ROOT, SOURCES
import re,json,csv,collections
idx=json.load(open(str(WORK / 'diagrams.json')))
def toks(s):return set(re.findall('[a-z0-9]+',s.lower()))-{'the','and','of','or','entity','type','subtype','code','symbol','set','value','icons','icon','modifiers'}
def kind(r):
 if r['table'] in ['B-I','1-3']:return 'common_m1'
 if r['table'] in ['B-II','1-4']:return 'common_m2'
 if len(r['code'])==6:return 'entity'
 m=re.search(r'(?:sector|modifier)\s*([12])',r['title'],re.I)
 return 'm'+m[1] if m else None
def app_scope(r):
 t=' '.join(r['text'].split())
 m=re.search(r'\b(one|single|two|three|four|five|1|2|3|4)\s+(?:\(\d\)\s+)?(?:anchor )?points?',t,re.I)
 dynamic=re.search(r'Static/Dynamic:\s*([SD])',t)
 if dynamic and dynamic[1]=='D' or re.search(r'requires N anchor points',t,re.I):return 'multipoint'
 if m:return 'point' if m[1].lower() in ['one','single','1'] else 'multipoint'
 if dynamic and dynamic[1]=='S':return 'point'
 return None

last_rules={}
for d in idx['app6e2']:
 if d['set']!='25' or len(d['code'])!=6:continue
 explicit=app_scope(d)
 if explicit:last_rules[d['table']]=(explicit,d['page'],d['code'])
 if d['table'] in last_rules:
  d['inherited_scope'],d['rule_page'],d['rule_code']=last_rules[d['table']]

overrides=json.load(open('tools/standards/drawing-overrides.json'))
results={}
for name,diags in idx.items():
 codes=list(csv.DictReader(open(f'tests/data/standards/{name}/codes.tsv'),delimiter='\t'));out=[];miss=[]
 for row in codes:
  override=next((o for o in overrides if o['document']==name and all(o[k]==row[k] for k in ['symbol_set','kind','code'])),None)
  cand=[r for r in diags if r['code']==row['code'] and (not r['set'] or r['set']==row['symbol_set']) and kind(r)==row['kind']]
  if not cand and row['kind'] in ['m1','m2']:
   cand=[r for r in diags if r['code']==row['code'] and r['set']==row['symbol_set'] and len(toks(row['name'])&toks(r['description']))/max(1,len(toks(row['name'])))>.8]
  cand.sort(key=lambda r:len(toks(row['name'])&toks(r['description']))/max(1,len(toks(row['name']))),reverse=True)
  if override:
   cand=[r for r in diags if r['code']==override['drawing_code'] and (not override['page'] or r['page']==override['page']) and override['name'].lower() in r['description'].lower()]
   assert len(cand)==1,(override,cand)
  found=cand[0] if cand else None
  row['drawing']=found
  if row['icon']=='no' or (found and re.search(r'reserved for hierarch|no associated (symbol|icon)|hierarchical explanation',found['text'],re.I)):row['scope']='hierarchy'
  elif row['code'] in ['00','000000']:row['scope']='empty'
  elif found:
   t=' '.join(found['text'].split())
   if name=='mil2525e1' and row['symbol_set'] in ['25','45','46'] and row['kind']=='entity':
    m=re.search(r'Draw Rules:\s*(\w+)',t)
    row['scope']='point' if m and m[1].startswith('Point') and not (row['symbol_set']=='25' and m[1] in ['Point12','Point17','Point18']) and not (row['symbol_set'] in ['45','46'] and m[1]=='Point5') else 'multipoint' if m else 'unclassified'
   elif row['symbol_set']=='25' and row['kind']=='entity':
    row['scope']=app_scope(found) or found.get('inherited_scope','unclassified')
   else:row['scope']='point'
  else:row['scope']='unclassified';miss.append(f"{row['symbol_set']} {row['kind']} {row['code']} {row['name']}")
  if row['symbol_set']=='25' and row['kind'] in ['m1','m2'] and found and re.search(r'Limited Access Area.*only|minefields.*mined areas.*only', ' '.join(found['text'].split()), re.I):row['scope']='multipoint'
  if name=='mil2525e1' and (row['symbol_set'],row['kind'],row['code']) in [('25','entity','140600'),('25','entity','160200'),('45','entity','110300')]:row['scope']='hierarchy'
  if name=='app6e2' and row['symbol_set']=='25' and row['kind']=='entity' and row['drawing'] and row['drawing']['table'] in ['8-23','8-26'] and row['scope'] != 'hierarchy':row['scope']='point'
  out.append(row)
 results[name]=out
 print(name,'missing diagrams',len(miss),collections.Counter(r['scope'] for r in out));print('\n'.join(miss[:100]))
json.dump(results,open(str(WORK / 'matched.json'),'w'),indent=2)
