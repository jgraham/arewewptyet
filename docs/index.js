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

async function render() {
    let runs = await getRuns();
    if (!runs) {
        return;
    }
    let runIds = runs.map(run => run.id);
    await Promise.all([setCount(runIds, "all", false),
                       setCount(runIds, "untraiged", true)]);
}

render();
