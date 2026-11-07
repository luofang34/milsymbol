from drawing_config import WORK, ROOT, SOURCES
import re,math,xml.etree.ElementTree as ET,base64,io
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.transformPen import TransformPen
from fontTools.svgLib.path import parse_path
from fontTools.misc.transform import Transform
from PIL import Image, ImageDraw
import numpy as np,potrace
SVG='{http://www.w3.org/2000/svg}';LINK='{http://www.w3.org/1999/xlink}href'
IDENTITY=Transform()

def matrix(text):
 m=IDENTITY
 for op,args in re.findall(r'(\w+)\(([^)]+)\)',text):
  v=[float(x) for x in re.split('[, ]+',args.strip())]
  if op=='matrix':n=Transform(*v)
  elif op=='translate':n=Transform(1,0,0,1,v[0],v[1] if len(v)>1 else 0)
  elif op=='scale':n=Transform(v[0],0,0,v[1] if len(v)>1 else v[0],0,0)
  else:raise ValueError(op)
  m=m.transform(n)
 return m

def transform_path(d,m):
 p=SVGPathPen(None,ntos=lambda x:format(x,'.6f').rstrip('0').rstrip('.') or '0');parse_path(d,TransformPen(p,m));return p.getCommands()

def bounds(d,m):
 p=BoundsPen(None);parse_path(d,TransformPen(p,m));return p.bounds

def point(p): return (p.x,p.y)

def image_paths(img):
 a=np.array(img.convert('RGB'));black=np.max(a,axis=2)<112
 bitmap=potrace.Bitmap(~black)
 contours=bitmap.trace(turdsize=0,alphamax=1,opticurve=True,opttolerance=.1)
 p=SVGPathPen(None)
 for curve in contours:
  p.moveTo(point(curve.start_point))
  for s in curve:
   if s.is_corner:p.lineTo(point(s.c));p.lineTo(point(s.end_point))
   else:p.curveTo(point(s.c1),point(s.c2),point(s.end_point))
  p.closePath()
 return p.getCommands()

def image_layers(img, single_color=None):
 """Trace separately coloured ink; antialias samples belong to the nearest ink."""
 a=np.array(img.convert('RGB')).astype(float)
 quantized=img.convert('RGB').quantize(colors=16)
 palette=quantized.getpalette(); candidates=[]
 for count,index in sorted(quantized.getcolors(),reverse=True):
  c=np.array(palette[index*3:index*3+3],dtype=float)
  if min(c)>230 or count<a.shape[0]*a.shape[1]*.002:continue
  if any(np.linalg.norm(c-k)<65 for k in candidates):continue
  # Intermediate samples between an ink and white are antialiasing, not ink.
  if any(np.linalg.norm(np.cross(255-c,255-k))/max(1,np.linalg.norm(255-k))<20 for k in candidates):continue
  candidates.append(c)
 if not candidates:return []
 colors=np.array([[255,255,255],*candidates])
 nearest=np.argmin(np.sum((a[:,:,None,:]-colors[None,None,:,:])**2,axis=3),axis=2)
 output=[]
 for i,c in enumerate(candidates,1):
  mask=nearest==i
  d=image_paths(Image.fromarray(np.where(mask,0,255).astype('uint8')))
  color=single_color if len(candidates)==1 and single_color else '#'+''.join(f'{round(v):02x}' for v in c)
  if d:output.append((d,color))
 return output

def white_interiors(img):
 """White-filled source icons occlude the frame fill inside closed black outlines."""
 a=np.array(img.convert('RGB'));ink=np.max(a,axis=2)<112
 regions=Image.fromarray(np.where(ink,0,255).astype('uint8'))
 # Padding makes every exterior region reachable from one flood-fill seed.
 padded=Image.new('L',(regions.width+2,regions.height+2),255)
 padded.paste(regions,(1,1));ImageDraw.floodfill(padded,(0,0),128)
 interior=np.array(padded)[1:-1,1:-1]==255
 return image_paths(Image.fromarray(np.where(interior,0,255).astype('uint8')))

def read(svg):
 root=ET.fromstring(svg);defs={e.attrib['id']:e for e in root.iter() if 'id' in e.attrib};out=[]
 def walk(e,m,style):
  tag=e.tag.removeprefix(SVG);a=e.attrib;m=m.transform(matrix(a.get('transform','')));st=style|{k:v for k,v in a.items() if k in ['fill','stroke','stroke-width','fill-rule','mask']}
  if tag=='defs':return
  if tag=='use':walk(defs[a[LINK][1:]],m.translate(float(a.get('x',0)),float(a.get('y',0))),st)
  elif tag=='path':
   if not a.get('d'):return
   b=bounds(a['d'],m)
   if b:out.append({'d':a['d'],'matrix':list(m),'bbox':b,'fill':st.get('fill','black'),'stroke':st.get('stroke'),'width':float(st.get('stroke-width',1))*math.sqrt(abs(m.xx*m.yy-m.xy*m.yx))})
  elif tag=='image':
   w,h=int(a['width']),int(a['height']);b=[*m.transformPoint((0,0)),*m.transformPoint((w,h))]
   data=a[LINK]
   if st.get('mask'):
    mask=defs[st['mask'].removeprefix('url(#').removesuffix(')')]
    images=list(mask.iter(SVG+'image'))
    for use in mask.iter(SVG+'use'):
     target=defs[use.attrib[LINK][1:]]
     assert target.tag==SVG+'image', 'unsupported non-image PDF soft mask'
     images.append(target)
    assert len(images)==1, 'unsupported compound PDF image mask'
    alpha=Image.open(io.BytesIO(base64.b64decode(images[0].attrib[LINK].split(',')[1]))).convert('L')
    source=Image.open(io.BytesIO(base64.b64decode(data.split(',')[1]))).convert('RGB')
    if source.size != alpha.size:
     # PDF soft masks have their own sample resolution over the same image box.
     alpha=alpha.resize(source.size,Image.Resampling.BILINEAR)
    source=Image.composite(source,Image.new('RGB',source.size,'white'),alpha)
    encoded=io.BytesIO();source.save(encoded,format='PNG')
    data='data:image/png;base64,'+base64.b64encode(encoded.getvalue()).decode()
   out.append({'image':data,'matrix':list(m),'bbox':b,'fill':'black','stroke':None,'width':0})
  elif tag in ['svg','g']: 
   for child in e:walk(child,m,st)
  elif tag in ['clipPath','title','desc']:return
  else:raise ValueError(tag)
 walk(root,IDENTITY,{})
 return out
