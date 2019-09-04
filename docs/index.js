async function getRuns() {
    let resp = await fetch("runs.json");
    let runs = await resp.json();
    return runs;
}

async function drawCharts() {
    let data = await getRuns();

    data.runs.sort((a, b) => new Date(a.date).getTime() >
                   new Date(b.date).getTime() ? -1 : 1);

    let latest = data.runs[0];
    document.getElementById("all").textContent = latest.all_failures.tests;
    document.getElementById("untriaged").textContent = latest.untriaged_failures.tests;

    function handleChartSelection(selection) {
        if (!selection.length) {
            return;
        }
        let idx = selection[0].row;
        let run = data.runs[idx];
        let link = `https://wpt.fyi/results/?label=master&product=firefox&product=chrome&product=safari&q=%28firefox%3A%21pass%26firefox%3A%21ok%29%20%28chrome%3Apass%7Cchrome%3Aok%29%20%28safari%3Apass%7Csafari%3Aok%29&sha=${run.revision}`;
        window.open(link);
    }

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

    google.visualization.events.addListener(testChart, 'select',
                                            () => handleChartSelection(testChart.getSelection()));

    var subtestChartOptions = {
        title: 'Fx-only subtest failures'
    };
    var subtestChart = new google.visualization.LineChart(document.getElementById('subtest_chart'));
    subtestChart.draw(subtestChartData, subtestChartOptions);

    google.visualization.events.addListener(subtestChart, 'select',
                                            () => handleChartSelection(subtestChart.getSelection()));

}


async function render() {
    google.charts.load('current', {'packages':['corechart']});
    google.charts.setOnLoadCallback(drawCharts);
}

render();
