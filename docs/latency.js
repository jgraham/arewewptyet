let updateRe = /.*Update web-platform-tests to ([0-9a-fA-F]+)/;

async function getCurrentLanding() {
    let resp = await fetch("https://bugzilla.mozilla.org/rest/bug?whiteboard=[wptsync%20landing]&status=NEW");
    let bugs = await resp.json();
    if (bugs.bugs.length === 0) {
        return null;
    }
    let filtered = [];

    // Also filter on summary
    bugs.bugs.filter(bug => {
        let changeset = bug.summary.match(updateRe);
        if (changeset !== null) {
            bug.wptrev = changeset[1];
            return true;
        }
        return false;
    });

    if (bugs.bugs.length > 1) {
        console.error("Found more than 1 in-progress landing");
        bugs.bugs.sort((a, b) => new Date(a.creation_time).getTime() >
                       new Date(b.creation_time).getTime() ? -1 : 1);
    }
    return bugs.bugs[0];
}

async function getGitHubPr(wptRev) {
    await new Promise(resolve => setTimeout(resolve, 100));
    let resp = await fetch(`https://api.github.com/repos/web-platform-tests/wpt/commits/${wptRev}/pulls`,
                           {headers: {accept: "application/vnd.github.groot-preview+json"}});
    if (resp.status == 403) {
        // Hit the GitHub rate limits
        return null;
    }
    let pr = await resp.json();
    return pr[0];
}

async function getCurrent() {
    let currentBug = await getCurrentLanding();
    if (currentBug === null) {
        return "No in-progress landing";
    } else {
        let pr = await getGitHubPr(currentBug.wptrev);
        if (pr === null) {
            rateLimited();
            return;
        }
        let prLandedAt = new Date(pr.closed_at);
        let latency = new Date() - prLandedAt;
        return `In-progress landing in bug ${currentBug.id}, current latency
${(latency / (1000 * 24 * 3600)).toLocaleString(undefined, {maximumFractionDigits: 0})} days`;
    }
};


async function getSyncPoints() {
    setStatus("Getting latencies");
    let resp = await fetch("landings.json");
    let syncPoints = await resp.json();
    return syncPoints;
}

function setStatus(text) {
    document.getElementById('latency_chart').textContent = `Loading: ${text}…`;
}


async function drawCharts() {
    let data = await getSyncPoints();
    var chartData = new google.visualization.DataTable();
    chartData.addColumn('datetime', 'Sync Date');
    chartData.addColumn('number', 'Latency / days');
    for (let syncPoint of data.landings) {
        chartData.addRow([new Date(syncPoint.gecko_push_time * 1000),
                          (syncPoint.gecko_push_time - syncPoint.wpt_merge_time) / (24 * 3600)]);
    }
    var options = {
        title: 'wpt sync latency'
    };
    var chart = new google.visualization.LineChart(document.getElementById('latency_chart'));
    chart.draw(chartData, options);
}

async function render() {
    google.charts.load('current', {'packages':['corechart']});
    google.charts.setOnLoadCallback(drawCharts);
    document.getElementById("current").textContent = await getCurrent();
}

render();
