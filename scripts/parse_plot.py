#!/usr/bin/python3

import matplotlib.pyplot as plt
from lxml import etree
import sys
import pandas as pd

def get_segment_properties(root, seg):
    prefix = "/object[@name='" + seg + "']"
    label = root.xpath(prefix+"/property[@name='label']")[0].text
    seg_from = float(root.xpath(prefix+"/property[@name='from']")[0].text)
    seg_to = float(root.xpath(prefix+"/property[@name='to']")[0].text)
    nint = int(root.xpath(prefix+"/property[@name='n_intervals']")[0].text)
    prec = int(root.xpath(prefix+"/property[@name='precision']")[0].text)
    inv = bool(root.xpath(prefix+"/property[@name='invert']")[0].text)
    log = bool(root.xpath(prefix+"/property[@name='log_scaling']")[0].text)
    off = float(root.xpath(prefix+"/property[@name='grid_offset']")[0].text)    
    return {
    	'label' : label, 
    	'from' : seg_from, 
    	'to' : seg_to, 
        'n_intervals' : nint, 
        'precision' : prec, 
        'invert' : inv, 
        'log_scaling' : log, 
        'grid_offset' : off 
    }
    
def get_design_properties(root):
    prefix = "/plotgroup/object[@class='design']"
    split = root.xpath(prefix+"/property[@name='split']")[0].text
    v_ratio = root.xpath(prefix+"/property[@name='vertical_ratio']")[0].text
    h_ratio = root.xpath(prefix+"/property[@name='horizontal_ratio']")[0].text
    bg_color = root.xpath(prefix+"/property[@name='bg_color']")[0].text
    grid_color = root.xpath(prefix+"/property[@name='grid_color']")[0].text
    grid_width = float(root.xpath(prefix+"/property[@name='grid_width']")[0].text)
    grid_font = root.xpath(prefix+"/property[@name='font']")[0].text
    return {
    	'split' : split,
    	'vertical_ratio' : v_ratio,
    	'horizontal_ratio' : h_ratio,
    	'bg_color' : bg_color, 
    	'grid_color' : grid_color, 
        'grid_width' : grid_width, 
        'grid_font' : grid_font 
    }

def get_dimensions(root):
	prefix = "/plotgroup/object[@class='dimensions']"
	width = root.xpath(prefix+"/property[@name='width']")[0].text
	height = root.xpath(prefix+"/property[@name='height']")[0].text
	return {
		'width' : width,
		'height' : height
	}
	
def get_mapping_properties(mapping):
    name = mapping.get("name")
    m_type = mapping.get("type")
    props = { 
        'name' : name,
        'type' : m_type,
        'x' : mapping.xpath("property[@name='x']")[0].text,
        'y' : mapping.xpath("property[@name='y']")[0].text,
        'color' : mapping.xpath("property[@name='color']")[0].text 
    }
    if m_type == 'line':
        props['width'] = int(mapping.xpath("property[@name='width']")[0].text) 
        props['dash'] = int(mapping.xpath("property[@name='dash']")[0].text) 
    elif m_type == 'scatter':
        props['radius'] = float(mapping.xpath("property[@name='radius']")[0].text)
    elif m_type == 'area':
        props['ymax'] = mapping.xpath("property[@name='ymax']")[0].text 
        props['opacity'] = float(mapping.xpath("property[@name='opacity']")[0].text)
    elif m_type == 'bar':
        props['width'] = mapping.xpath("property[@name='width']")[0].text 
        props['height'] = mapping.xpath("property[@name='height']")[0].text 
    elif m_type == 'text':
        props['font'] = mapping.xpath("property[@name='font']")[0].text 
        props['text'] = mapping.xpath("property[@name='text']")[0].text 
    elif m_type == 'surface':
        props['z'] = mapping.xpath("property[@name='z']")[0].text 
        props['final_color'] = mapping.xpath("property[@name='final_color']")[0].text 
        props['z_min'] = float(mapping.xpath("property[@name='z_min']")[0].text) 
        props['z_max'] = float(mapping.xpath("property[@name='z_max']")[0].text)
        props['opacity'] = float(mapping.xpath("property[@name='opacity']")[0].text)
    else:
        raise Exception('Unrecognized type')
    return props
    
def pool_mappings(root):
    map_props = []
    mappings = root.findall(".//object[@class='mapping']")
    for m in mappings:
        props = get_mapping_properties(m)
        map_props.append(props)
    return map_props
    
def load_layout_root(path):
	try:
		with open(path) as f:
		    lines = f.readlines()
		    full = ""
		    for line in lines:
		        if "<?" in line:
		            pass
		        else:
		            full += line
		    root = etree.XML(full)
		    return root
	except Exception(e):
		print(f"XML parsing error: {}", e)
        sys.exit(-1)
                    
def adjust_labels(area, ax):
	plt.locator_params(axis='x', nbins=areas[i]['x']['n_intervals'])
    plt.locator_params(axis='y', nbins=areas[i]['y']['n_intervals'])
    ax.grid(color=design['grid_color'], linewidth=design['grid_width'])
    ax.set_facecolor(design['bg_color'])
    ax.set_xlabel(areas[i]['x']['label'])
    ax.set_ylabel(areas[i]['y']['label'])
    ax.set_xlim(areas[i]['x']['from'], areas[i]['x']['to'])
    ax.set_ylim(areas[i]['y']['from'], areas[i]['y']['to'])
    if areas[i]['x']['invert']:
        ax.invert_xaxis()
    if areas[i]['y']['invert']:
        ax.invert_yaxis()
    if areas[i]['x']['log_scaling']:
        ax.set_xscale('log')
    if areas[i]['y']['log_scaling']:
        ax.set_yscale('log')
        
def search_data(data, mapping, name):
	col = None
	for d in data:
		try:
			col = d[mapping[name]]
			break
		except:
			continue
	return col
	
def draw_area(area, ax, data):
	for mapping in area['mappings']:
        if mapping['type'] == 'line':
            ax.plot(
                search_data(data, mapping, 'x'), 
                search_data(data, mapping, 'y'), 
                '-', 
                color=search_data(data, mapping, 'color'),, 
                markersize=search_data(data, mapping, 'radius'),,
                linewidth=search_data(data, mapping, 'width'),
            )
        elif mappping['type'] == 'scatter':
            ax.plot(
                search_data(data, mapping, 'x'),, 
                search_data(data, mapping, 'y'),, 
                'o', 
                color=search_data(data, mapping, 'color'), 
                markersize=search_data(data, mapping, 'radius'),
            )
        elif mappping['type'] == 'bar':
            #ax.bar(
            #    x=search_data(data, mapping, 'x'), 
            #    height=search_data(data, mapping, 'height'),
            #    width=data[mapping['width']], 
            #    bottom=data[mapping['y']],
            #    color=mapping['color']
            #)
            raise Exception("Unimplemented")
        elif mappping['type'] == 'text':
            for (x, y, t) in zip(data[mapping['x']], data[mapping['y']], data[mapping['text']]):
                ax.annotate(xy=(x, y), text=t)
        elif mappping['type'] == 'area':
            ax.fill_between(
                data[mapping['x']], 
                data[mapping['y']], 
                data[mapping['ymax']], 
                color=mapping['color']
            )      
        elif mappping['type'] == 'surface':
            raise Exception("Unimplemented")
        else:
            raise Exception("Unknown mapping type")

def define_figure_split(design):
	if design.split == "Unique":
    	return f.subplots(1,1)
    elif design.split == "Vertical"
    	raise Exception("Unimplemented split")
    elif design.split == "Horizontal"
    	raise Exception("Unimplemented split")
    elif design.split == "ThreeLeft":
    	raise Exception("Unimplemented split")
    elif design.split == "ThreeTop":
    	raise Exception("Unimplemented split")
    elif design.split == "ThreeRight":
    	raise Exception("Unimplemented split")
    elif design.split == "ThreeBottom":
    	raise Exception("Unimplemented split")
    elif design.split == "Four":
    	raise Exception("Unimplemented split")
    else:
    	print("Unknown split parameter")
    	sys.exit(-1)

def draw_plot(design, dimensions, areas, data, dst):
    with plt.style.context("seaborn-whitegrid"):
        f = plt.figure(figsize=(8, 6))
        axes = define_figure_split(design)

        for (i, ax) in enumerate(axes):
        	adjust_labels(areas[i], ax)
		    draw_area(areas[i], ax, data)
		            
        f.savefig(dst)
 
def join_axes(axes):

# Read layout path (required)
def read_layout():
	try:
    	ix_layout = sys.argv.index('-o')
    	layout = sys.argv[ix_layout + 1]
    	return layout
    except ValueError:
    	print("Missing -l (layout) keyword
    	sys.exit(-1)
	except IndexError:
		print("Missing layout argument")
		sys.exit(-1)

# Read data from colon-separated CSV paths.
def read_data():
	try:
    	ix_data = sys.argv.index('-d')
    	data = sys.argv[ix_data + 1]
    	return data
    except ValueError:
    	print("Missing -d (data) keyword
    	sys.exit(-1)
	except IndexError:
		print("Missing layout argument")
		sys.exit(-1)
	
def read_output():
	try:
		ix_out = sys.argv.index('-o')
		out_path = sys.argv[ix_out + 1]
		return out_path
	except ValueError:
		print("Missing -o (output)")
		sys.exit(-1)
	except IndexError:
		print("Missing plot output path")
		sys.exit(-1)
		
def get_area_list(layout_root):
	prefix = 
	areas = []
	for area_xml in layout_root.xpath("/plotgroup/object[@class='plotarea']"):
		x = get_segment_properties(area_xml, 'x')
    	y = get_segment_properties(area_xml, 'y')
    	mappings = pool_mappings(area_xml)
    	areas.append({ 'x' : x, 'y' : y, 'mappings' : mappings })
   	areas
   	
if __name__ == "__main__":
    fname = read_layout()
    data_path = read_data()
	output_path = read_output()
	
    layout_root = load_layout_root(fname)
    design = get_design_properties(layout_root)
    dimensions = get_dimensions(layout_root)
    areas = get_area_list(layout_root)
    data = []
    for path in data_path.split(':'):
    	data.push(pd.read_csv(path))
    draw_plot(design, dimensions, areas, data, dst)
        
