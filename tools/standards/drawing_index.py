from drawing_config import WORK, ROOT, SOURCES
import pymupdf as fitz,re,json,collections
sources={'mil2525e1':SOURCES['mil2525e1'],'app6e2':SOURCES['app6e2']}
result={}
for name,src in sources.items():
 doc=fitz.open(src);out=[];tables={};lastcap=''
 for i,page in enumerate(doc):
  if name=='mil2525e1' and i<160:continue
  if name=='app6e2' and (i<42 or i>=744):continue
  txt=page.get_text();captions=[];section_title=''
  for block in page.get_text('dict')['blocks']:
   for line in block.get('lines',[]):
    text=''.join(span['text'] for span in line['spans']).strip()
    section=re.match(r'C\.\d+\.\d+\s+(.*?)\.\s',text)
    if section:section_title=section[1]
    reference=re.search(r'\bTable (C-[IVX]+) lists\b',text)
    if name=='mil2525e1' and reference and section_title:
     captions.append((line['bbox'][1],reference[1],section_title))
    m=re.match(r'(?:TABLE|Table)\s+([A-Z\d]+(?:[-–][A-Z\d]+)+)[.:]?\s+(.*)',text)
    if m:captions.append((line['bbox'][1],m.group(1).replace('–','-'),m.group(2)))
  captions.sort()
  if not captions and not lastcap:continue
  cap=lastcap if lastcap else captions[0][1:]
  drawings=page.get_drawings()
  ys=[]
  for d in drawings:
   r=d['rect']
   if r.height<1.6 and r.width>60 and r.x0<120:ys.append((r.y0+r.y1)/2)
   for item in d['items']:
    if item[0]=='l':
     a,b=item[1:];
     if abs(a.y-b.y)<.5 and abs(a.x-b.x)>60 and min(a.x,b.x)<120:ys.append(a.y)
  clean=[]
  for y in sorted(ys):
   if not clean or y-clean[-1]>2:clean.append(y)
  for top,bottom in zip(clean,clean[1:]):
   if bottom-top<20:continue
   above=[c for c in captions if c[0]<top]
   table,title=above[-1][1:] if above else cap
   tables.setdefault(table,[i+1,title])
   # Left cells contain identification; the full band retains draw rules and scope.
   desc=page.get_text(clip=fitz.Rect(45,top,255,bottom));full=page.get_text(clip=fitz.Rect(45,top,550,bottom))
   setmatch=r'Symbol Set\s*(?:Code|Value)?:?\s*(\d{2})'
   withoutset=re.sub(setmatch,'',desc,flags=re.I)
   code=re.findall(r'(?:Value|Code):?\s*([0-9A-F]{2,6})(?![0-9A-F])',withoutset,re.I)
   if not code and table in ['1-3','1-4']:
    code=re.findall(r'^\s*(1[0-9]{2})\s',desc)
   if not code:continue
   sets=re.findall(setmatch,desc,re.I)
   rulesx=330 if name=='app6e2' else 450
   boundaries=[r['rect'].y0 for r in drawings if r['rect'].height<1.6 and r['rect'].width>60 and r['rect'].x0<rulesx<r['rect'].x1]
   middle=(top+bottom)/2
   rt=max([y for y in boundaries if y<middle],default=top)
   rb=min([y for y in boundaries if y>middle],default=bottom)
   ruletext=page.get_text(clip=fitz.Rect(260 if name=='app6e2' else 390,rt,405 if name=='app6e2' else 545,rb))
   full+='\n'+ruletext
   if len(set(code))>1:print('MULTI',name,i+1,code)
   out.append({'page':i+1,'printed':i+1-(17 if name=='mil2525e1' else 34),'table':table,'title':title,'y':[round(top,3),round(bottom,3)],'code':code[0],'set':sets[0] if sets else '', 'description':desc,'text':full})
  if captions:lastcap=captions[-1][1:]
 result[name]=out
 print(name,len(out),'tables',tables)
json.dump(result,open(str(WORK / 'diagrams.json'),'w'),indent=2)
