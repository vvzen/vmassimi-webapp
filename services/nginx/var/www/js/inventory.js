// Collapse a node and all its children
function collapse(d) {
    if (d.children) {
        d._children = d.children
        d._children.forEach(collapse)
        d.children = null
    }
}

// Set the dimensions and margins of the diagram
let margin = {
        top: 20,
        right: 90,
        bottom: 30,
        left: 90
    },
    width = 1920 - margin.left - margin.right,
    height = 720 - margin.top - margin.bottom;

// append the svg object to the body of the page
// appends a 'group' element to 'svg'
// moves the 'group' element to the top left margin
let svg = d3.select('#graph').append('svg')
    //let svg = d3.select('body').append('svg')
    .attr('width', width + margin.right + margin.left)
    .attr('height', height + margin.top + margin.bottom)
    .append('g')
    .attr('transform', `translate(${margin.left},${margin.top})`);

let i = 0;
let transitionDuration = 750;

// declares a tree layout and assigns the size
let treemap = d3.tree().size([height, width]);
let root;

d3.json('/app/api/inventory').then((data) => {

    console.log(data);

    // Assigns parent, children, height, depth
    root = d3.hierarchy(data, (d) => {

        // Sort all children
        d.children.sort((a, b) => {
            return a.name.replace("_", "").localeCompare(b.name.replace("_", ""));
        });

        return d.children;

    });
    root.x0 = height / 2;
    root.y0 = 0;

    // Collapse after the second level
    root.children.forEach(collapse);

    update(root);

});

// Creates a curved (diagonal) path from parent to the child nodes
function diagonal(s, d) {

  path = `M ${s.y} ${s.x}
          C ${(s.y + d.y) / 2} ${s.x},
            ${(s.y + d.y) / 2} ${d.x},
            ${d.y} ${d.x}`
  return path
}

function onClick(event, d) {

  // Toggle children on click
  console.log("Clicked on", d);
  console.log(`Clicked on ${d.data.name}`);

  if (!d.data.is_file){

    if (d.children) {
      d._children = d.children;
      d.children = null;
    }
    else {
      d.children = d._children;
      d._children = null;
    }
    
  }
  else {
    // TODO: show a modal here 
    alert(`Something important to do with ${d.data.file_path}`)

    // TODO: find the path back to root
  }

  update(d);
}

function update(source) {

    // Assigns the x and y position for the nodes
    let treeData = treemap(root);

    // Compute the new tree layout.
    let nodes = treeData.descendants(),
        links = treeData.descendants().slice(1);
    nodes.forEach((d) => {
        // Normalize for fixed-depth.
        d.y = d.depth * 180
    });
    // ****************** Nodes section ***************************
    // Update the nodes...
    let node = svg.selectAll('g.node')
        .data(nodes, (d) => d.id || (d.id = ++i));
    // Enter any new modes at the parent's previous position.
    let nodeEnter = node.enter()
        .append('g')
        .attr('class', 'node')
        .attr('transform', (d) => {
            return `translate(${source.y0}, ${source.x0})`;
        })
        .on('click', onClick);

    // Add Circle for the nodes
    nodeEnter.append('circle')
        .attr('class', 'node')
        .attr('r', 1e-6);

    // Add labels for the nodes
    nodeEnter.append('text')
        .attr('dy', '.25em')
        .attr('x', function(d) {
            return d.children || d._children ? -13 : 13;
        })
        .attr('text-anchor', function(d) {
            return d.children || d._children ? 'end' : 'start';
        })
        .text(function(d) {
            return d.data.name;
        });

    // UPDATE
    let nodeUpdate = nodeEnter.merge(node);
    // Transition to the proper position for the node
    nodeUpdate.transition()
        .duration(transitionDuration)
        .attr('transform', function(d) {
            return `translate(${d.y},${d.x})`;
        });

    // Update the node attributes and style
    nodeUpdate.select('circle.node')
        .attr('r', 8)
        .style('stroke', (d) => {
            return 'none';
        })
        .style('fill', function(d) {

            let targetColor = 'lightsteelblue';
            if (d.data.is_file){
              targetColor = '#759465';
            }
            else if (d.children){
              targetColor = '#fff';
            }

            return targetColor;
        })
        .attr('cursor', 'pointer');

    // Remove any exiting nodes
    let nodeExit = node.exit().transition()
        .duration(transitionDuration)
        .attr('transform', function(d) {
            return `translate(${source.y}, ${source.x})`;
        })
        .remove();
    // On exit reduce the node circles size to 0
    nodeExit.select('circle')
        .attr('r', 1e-6);
    // On exit reduce the opacity of text labels
    nodeExit.select('text')
        .style('fill-opacity', 1e-6);

    // ****************** links section ***************************
    // Update the links...
    let link = svg.selectAll('path.link')
        .data(links, function(d) {
            return d.id;
        });

    // Enter any new links at the parent's previous position.
    let linkEnter = link.enter().insert('path', 'g')
        .attr('class', 'link')
        .attr('d', function(d) {
            let o = {
                x: source.x0,
                y: source.y0
            }
            return diagonal(o, o)
        });
    // UPDATE
    let linkUpdate = linkEnter.merge(link);
    // Transition back to the parent element position
    linkUpdate.transition()
        .duration(transitionDuration)
        .attr('d', function(d) {
            return diagonal(d, d.parent)
        });
    // Remove any exiting links
    let linkExit = link.exit().transition()
        .duration(transitionDuration)
        .attr('d', function(d) {
            let o = {
                x: source.x,
                y: source.y
            }
            return diagonal(o, o)
        })
        .remove();
    // Store the old positions for transition.
    nodes.forEach(function(d) {
        d.x0 = d.x;
        d.y0 = d.y;
    });
}
