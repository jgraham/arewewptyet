async function getRuns() {
    let resp = await fetch("https://wpt.fyi/api/runs?aligned&product=chrome[experimental]&product=firefox[experimental]&product=safari[experimental]&label=master",
                           {headers: {"Content-Type": "application/json"}});
    if (resp.status !== 200) {
        return null;
    }
    if (resp.status !== 200) {
        return null;
    }
    return await resp.json();
}

async function geckoOnlyFailures(runIds, untriaged) {
    let body = {run_ids: runIds,
                query: {
                    "and":[
                        {"not":{"browser_name":"firefox","status":"PASS"}},
                        {"not":{"browser_name":"firefox","status":"OK"}},
                        {"or":[{"browser_name":"chrome","status":"PASS"},
                               {"browser_name":"chrome","status":"OK"}]},
                        {"or":[{"browser_name":"safari","status":"PASS"},
                               {"browser_name":"safari","status":"OK"}]}]}};
    if (untriaged) {
        body.query.and.push({"not": {"link": "bugzilla.mozilla.org"}});
    }

    let resp = await fetch("https://wpt.fyi/api/search?label=master&product=chrome%5Bexperimental%5D&product=firefox%5Bexperimental%5D&product=safari%5Bexperimental%5D",
                           {body: JSON.stringify(body),
                            method: "POST",
                            headers: {"Content-Type": "application/json"},
                            mode: "cors"});
    if (resp.status !== 200) {
        return null;
    }
    return await resp.json();
}

async function setCount(runIds, id, untriaged) {
    let data = await geckoOnlyFailures(runIds, untriaged);
    if (data !== null) {
        document.getElementById(id).textContent = data.results.length;
    }
}

async function getRuns() {
    let resp = await fetch("runs.json");
    let runs = await resp.json();
    return runs;

}

async function drawCharts() {
    let data = await getRuns();
    var testChartData = new google.visualization.DataTable();
    testChartData.addColumn('datetime', 'Run Date');
    testChartData.addColumn('number', 'Fx-only test failures (all)');
    testChartData.addColumn('number', 'Fx-only test failures (untriaged)');

    var subtestChartData = new google.visualization.DataTable();
    subtestChartData.addColumn('datetime', 'Run Date');
    subtestChartData.addColumn('number', 'Fx-only subtest failures (all)');
    subtestChartData.addColumn('number', 'Fx-only subtest failures (untriaged)');

    for (let run of data.runs) {
        testChartData.addRow([new Date(run.date), run.all_failures.tests, run.untriaged_failures.tests]);
        subtestChartData.addRow([new Date(run.date), run.all_failures.subtests, run.untriaged_failures.subtests]);
    }

    var testChartOptions = {
        title: 'Fx-only test failures'
    };
    var testChart = new google.visualization.LineChart(document.getElementById('test_chart'));
    testChart.draw(testChartData, testChartOptions);

    var subtestChartOptions = {
        title: 'Fx-only subtest failures'
    };
    var subtestChart = new google.visualization.LineChart(document.getElementById('subtest_chart'));
    subtestChart.draw(subtestChartData, subtestChartOptions);
}

async function render() {
    google.charts.load('current', {'packages':['corechart']});
    google.charts.setOnLoadCallback(drawCharts);
    let runs = await getRuns();
    if (!runs) {
        return;
    }
    let runIds = runs.runs[runs.runs.length - 1].run_ids;
    await Promise.all([setCount(runIds, "all", false),
                       setCount(runIds, "untraiged", true)]);
}

render();
