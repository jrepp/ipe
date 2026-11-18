(() => {
  const routes = [
    {
      id: "projects-p2",
      planned: 2,
      stations: [
        { label: ["Connect", "source"], icon: "M-7 -7h14v5H-7zM-7 2h14v5H-7zM-4 -4.5h1M-4 4.5h1" },
        { label: ["Observe"], icon: "M-9 0q9 -11 18 0q-9 11 -18 0zM0 -2a2 2 0 1 0 0 4a2 2 0 1 0 0 -4" },
        { label: ["Assess", "evidence"], icon: "M-6 -8h12v16H-6zM-3 0l2 2l4 -4" },
        { label: ["Publish", "facts"], icon: "M-7 2v6H7V2M0 4V-8M-4 -4l4 -4l4 4" },
      ],
    },
    {
      id: "projects-p4",
      stations: [
        { label: ["Decision"], icon: "M0 -9L9 0 0 9 -9 0zM0 5V0M-3 -3l3 3l3 -3" },
        { label: ["Evidence", "links"], icon: "M-1 -4l2 -2a3.5 3.5 0 0 1 5 5L4 1M1 4l-2 2a3.5 3.5 0 0 1 -5 -5l2 -2M-2 2l4 -4" },
        { label: ["Time", "limits"], icon: "M0 -8a8 8 0 1 0 0 16a8 8 0 1 0 0 -16M0 -4v4l3 2" },
        { label: ["Audit"], icon: "M-6 -7h12v15H-6zM-3 -3h6M-3 0h6M-3 3h4" },
      ],
    },
    {
      id: "projects-p5",
      stations: [
        { label: ["Explain", "product"], icon: "M-8 -6h16v10H0l-4 4V4h-4zM-4 -2h8M-4 1h5" },
        { label: ["Check", "docs"], icon: "M-6 -8h12v16H-6zM-3 0l2 2l4 -4" },
        { label: ["Publish to", "GitHub Pages"], icon: "M-8 -6H8V6H-8zM-8 -2H8M-4 2h8" },
      ],
    },
  ];

  for (const { id, planned, stations } of routes) {
    const svg = d3.select(`#${id} > svg`);
    const position = index => `${100 * (index + 0.5) / stations.length}%`;

    svg.selectAll(".flow-segment")
      .data(stations.slice(1))
      .join("line")
      .attr("class", (_, index) => index === planned ? "flow-segment planned" : "flow-segment")
      .attr("x1", (_, index) => position(index))
      .attr("x2", (_, index) => position(index + 1))
      .attr("y1", 24)
      .attr("y2", 24);

    svg.selectAll(".flow-arrow")
      .data(stations.slice(1))
      .join("svg")
      .attr("class", "flow-arrow")
      .attr("x", (_, index) => position(index + 0.5))
      .attr("y", 24)
      .attr("width", 1)
      .attr("height", 1)
      .append("path")
      .attr("class", "flow-mark")
      .attr("d", "M-2 -4l4 4-4 4");

    const station = svg.selectAll(".flow-node")
      .data(stations)
      .join("svg")
      .attr("class", "flow-node")
      .attr("x", (_, index) => position(index))
      .attr("y", 24)
      .attr("width", 1)
      .attr("height", 1);

    station.append("circle").attr("class", "flow-station").attr("r", 17);
    station.append("path").attr("class", "flow-mark").attr("d", node => node.icon);
    station.append("text")
      .attr("class", "flow-label")
      .selectAll("tspan")
      .data(node => node.label)
      .join("tspan")
      .attr("x", 0)
      .attr("y", (_, index) => 36 + 15 * index)
      .text(line => line);
  }
})();
