#!/usr/bin/env python3
# Скрипт восстановления изменений в хронологическом порядке

import shutil
from pathlib import Path
import os

def restore_changes():
    changes = [
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-663df50e\h1PY.md',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\implementation_plan\README.md',
            'timestamp': '2025-07-29 01:08:45.581000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-663df50e\GekM.md',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\implementation_plan\README.md',
            'timestamp': '2025-07-29 01:08:58.895000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\436fc7e2\IAKW',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\.gitignore',
            'timestamp': '2025-07-29 02:48:39.438000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\2a10d16a\uUoy.md',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\docs\ARCHITECTURE.md',
            'timestamp': '2025-07-29 16:45:59.396000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-4c3113c5\wyur.md',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\CLAUDE.md',
            'timestamp': '2025-07-29 20:57:51.198000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-4c3113c5\dcsA.md',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\CLAUDE.md',
            'timestamp': '2025-07-30 00:25:00.693000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-4c3113c5\BoSP.md',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\CLAUDE.md',
            'timestamp': '2025-07-30 01:19:06.838000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-4c3113c5\971o.md',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\CLAUDE.md',
            'timestamp': '2025-07-30 01:26:29.759000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-7d007171\i9h5.md',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\отчет.md',
            'timestamp': '2025-08-06 03:17:04.531000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\79fa39d5\ZcKf.md',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\план.md',
            'timestamp': '2025-08-06 03:19:32.308000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\79fa39d5\eZ4z.md',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\план.md',
            'timestamp': '2025-08-06 03:22:17.102000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-5ada713c\mBVO.py',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\scripts\fix_unwraps.py',
            'timestamp': '2025-08-06 03:24:33.686000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-190b23eb\SCbu.yml',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\.github\workflows\ci.yml',
            'timestamp': '2025-08-06 03:25:04.044000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-776e5e9a\kfSi.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\tests\integration_basic.rs',
            'timestamp': '2025-08-06 03:25:39.816000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\4a183b75\J0ly.py',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\scripts\enable_real_models.py',
            'timestamp': '2025-08-06 03:26:46.223000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-73b6cd\Mt0W.ps1',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\scripts\fix_unwraps.ps1',
            'timestamp': '2025-08-06 03:28:21.943000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-73b6cd\iX3Y.ps1',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\scripts\fix_unwraps.ps1',
            'timestamp': '2025-08-06 03:28:41.476000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-523594e0\VM8y.toml',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\Cargo.toml',
            'timestamp': '2025-08-06 03:29:30.843000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-e49b1fa\OADe.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\src\embeddings_bge_m3.rs',
            'timestamp': '2025-08-06 03:29:55.785000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\7d9e81af\fCYq.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\src\embeddings_qwen3.rs',
            'timestamp': '2025-08-06 03:30:42.417000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-565bd877\Tebc.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\src\lib.rs',
            'timestamp': '2025-08-06 03:30:49.544000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-565bd877\R8Cz.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\src\lib.rs',
            'timestamp': '2025-08-06 03:30:56.147000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-e49b1fa\OXy5.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\src\embeddings_bge_m3.rs',
            'timestamp': '2025-08-06 03:31:09.267000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\7d9e81af\iZaN.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\src\embeddings_qwen3.rs',
            'timestamp': '2025-08-06 03:31:35.916000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\7d9e81af\D9BD.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\src\embeddings_qwen3.rs',
            'timestamp': '2025-08-06 03:31:49.003000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\7d9e81af\QoGL.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\src\embeddings_qwen3.rs',
            'timestamp': '2025-08-06 03:34:25.104000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\1f7a690\meD9.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\src\lib.rs',
            'timestamp': '2025-08-08 10:15:16.905000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-598901ff\SSGq.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\src\orchestration\mod.rs',
            'timestamp': '2025-08-08 10:15:16.953000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\61dea27\B0GX.toml',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\Cargo.toml',
            'timestamp': '2025-08-08 10:15:16.996000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-6f95e0e0\vXjS.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\src\di\memory_configurator.rs',
            'timestamp': '2025-08-08 10:15:17.034000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\31afbb71\ysQs.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\src\di\mod.rs',
            'timestamp': '2025-08-08 10:15:17.071000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-2dce6768\BuXg.toml',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\Cargo.toml',
            'timestamp': '2025-08-08 10:15:17.103000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-48e39c2\40in.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\src\di_compatibility_stub.rs',
            'timestamp': '2025-08-08 10:15:17.152000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\56b99b37\3kCf.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\src\simple_di\builder.rs',
            'timestamp': '2025-08-08 10:15:17.230000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-37c17827\97i6.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\src\utils\error_utils.rs',
            'timestamp': '2025-08-08 10:15:17.273000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-2d5dfe17\bX9O.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\src\hnsw_index\index.rs',
            'timestamp': '2025-08-08 10:15:17.307000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\6e668803\Gnge.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\src\cache_lru.rs',
            'timestamp': '2025-08-08 10:15:17.397000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-f66706c\xC3O.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\todo\src\store_v2.rs',
            'timestamp': '2025-08-08 10:15:17.433000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\5fa296b7\H79u.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\todo\src\types.rs',
            'timestamp': '2025-08-08 10:15:17.460000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\7025a42b\BH2y.md',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\README.md',
            'timestamp': '2025-08-08 10:15:17.511000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-1d29bd18\dsgh.md',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\docs\QUICKSTART.md',
            'timestamp': '2025-08-08 10:15:17.544000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\592806ef\FIIJ.md',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\docs\PROJECT_VISION.md',
            'timestamp': '2025-08-08 10:15:17.571000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-15acd913\M38G.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\domain\src\lib.rs',
            'timestamp': '2025-08-08 10:15:17.604000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\f834b27\Wj1Q.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\application\src\lib.rs',
            'timestamp': '2025-08-08 10:15:17.639000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\789073ee\Gb2N.toml',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\Cargo.toml',
            'timestamp': '2025-08-08 10:15:17.668000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\409a0e29\i9JR.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\main.rs',
            'timestamp': '2025-08-08 10:15:17.702000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\56a926a7\EwIz.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\commands\mod.rs',
            'timestamp': '2025-08-08 10:15:17.801000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-28df71c3\xIyz.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\commands\memory_stub.rs',
            'timestamp': '2025-08-08 10:15:17.926000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\4fef524e\oTOx.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\health_checks.rs',
            'timestamp': '2025-08-08 10:15:17.967000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-65f2f6fe\XIdD.toml',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\todo\Cargo.toml',
            'timestamp': '2025-08-08 10:15:18.436000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\78a5c91f\q4G6.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\unified_agent_v2.rs',
            'timestamp': '2025-08-08 10:15:18.626000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\118617d3\fXCN.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\handlers\chat_handler.rs',
            'timestamp': '2025-08-08 10:15:18.873000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-48da4c04\di5O.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\tools\src\lib.rs',
            'timestamp': '2025-08-08 10:15:19.047000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-4773a339\0GCW.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\tools\src\mcp.rs',
            'timestamp': '2025-08-08 10:15:19.189000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\3e50d2ae\gtE6.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\commands\tools.rs',
            'timestamp': '2025-08-08 10:15:19.238000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\55145800\XUvj.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\commands\smart.rs',
            'timestamp': '2025-08-08 10:15:19.280000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\13a98ebe\6bop.toml',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\application\Cargo.toml',
            'timestamp': '2025-08-08 10:15:19.431000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\ace4aa8\MfO7.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\todo\src\service_v2.rs',
            'timestamp': '2025-08-08 10:15:19.472000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\8371340\h8Nd.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\util.rs',
            'timestamp': '2025-08-08 10:15:19.506000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\607ee01b\r83x.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\commands\tasks.rs',
            'timestamp': '2025-08-08 10:15:19.542000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\746a3057\QqkM.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\lib.rs',
            'timestamp': '2025-08-08 10:15:19.575000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\3e7e10aa\kzDs.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\todo\tests\test_graph.rs',
            'timestamp': '2025-08-08 10:15:19.613000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-4f62de06\dkJZ.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\tests\test_services_resilience.rs',
            'timestamp': '2025-08-08 10:15:19.649000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\1bbe1d74\bjpe.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\tests\test_commands_memory.rs',
            'timestamp': '2025-08-08 10:15:19.687000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-607bbc8c\tzLS.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\tests\test_services_intent_analysis.rs',
            'timestamp': '2025-08-08 10:15:19.722000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-7da10492\wovT.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\status_tests.rs',
            'timestamp': '2025-08-08 10:15:19.756000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-7309ada4\Hs4r.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\tests\test_memory_integration.rs',
            'timestamp': '2025-08-08 10:15:19.791000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-4158b1e3\WkdQ.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\tests\test_services_request_routing.rs',
            'timestamp': '2025-08-08 10:15:19.820000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-e88c1d9\vUyO.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\tests\test_agent.rs',
            'timestamp': '2025-08-08 10:15:19.854000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-2de3d891\TzFK.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\tests\test_services_llm_communication.rs',
            'timestamp': '2025-08-08 10:15:19.884000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\3dd8690\KEVM.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\tests\test_tools_cli.rs',
            'timestamp': '2025-08-08 10:15:19.913000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\1c3d157d\4ac2.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\tests\test_tasks_cli.rs',
            'timestamp': '2025-08-08 10:15:19.941000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\57da12e2\1WpM.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\tests\test_smart_cli.rs',
            'timestamp': '2025-08-08 10:15:19.993000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\701cb635\Q5ze.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\agent_tests.rs',
            'timestamp': '2025-08-08 10:15:20.027000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-e808d0\sGLV.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\handler_registry.rs',
            'timestamp': '2025-08-08 10:15:20.514000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\67f3e108\8eU4.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\refactored_unified_agent.rs',
            'timestamp': '2025-08-08 10:15:20.584000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\6c46fc02\j0Ix.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\handlers\tools_handler.rs',
            'timestamp': '2025-08-08 10:15:20.628000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\1c49120e\o9mE.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\handlers\admin_handler.rs',
            'timestamp': '2025-08-08 10:15:20.661000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\411bccca\00Zn.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\handlers\memory_handler.rs',
            'timestamp': '2025-08-08 10:15:20.710000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-5b1f7043\9msc.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\handlers\performance_monitor.rs',
            'timestamp': '2025-08-08 10:15:20.767000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\4ecdc42c\6u4H.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\strategies\intent_strategies.rs',
            'timestamp': '2025-08-08 10:15:20.851000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\6fd5e6a\kozQ.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\strategies\circuit_breaker.rs',
            'timestamp': '2025-08-08 10:15:20.920000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-1d4a032f\Oqrx.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\strategies\response_strategies.rs',
            'timestamp': '2025-08-08 10:15:20.989000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\551c6833\ormL.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\performance_tracker.rs',
            'timestamp': '2025-08-08 10:15:21.056000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\3d0a0b6b\dQOW.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\tests\test_commands_models.rs',
            'timestamp': '2025-08-08 10:15:21.142000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\436fc7e2\tsLk',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\.gitignore',
            'timestamp': '2025-08-08 10:15:21.196000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-51146f26\slQo.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\llm\src\providers\mod.rs',
            'timestamp': '2025-08-08 10:15:21.259000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-245fa8b5\kcvH.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\src\embeddings_cpu.rs',
            'timestamp': '2025-08-08 10:15:21.322000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\45bb9cee\XPVf.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\tests\test_tokenizer_comprehensive.rs',
            'timestamp': '2025-08-08 10:15:21.385000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\103ba59f\S7kt.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\todo\tests\test_types.rs',
            'timestamp': '2025-08-08 10:15:21.454000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\37994ad\0LYm.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\todo\tests\test_extended.rs',
            'timestamp': '2025-08-08 10:15:21.508000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-96f4729\EzOg.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\test_unified_di_container.rs',
            'timestamp': '2025-08-08 10:15:21.565000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\7169356\HyD5.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\mod.rs',
            'timestamp': '2025-08-08 10:15:21.621000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-34d72d8f\jTaz.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\integration\mod.rs',
            'timestamp': '2025-08-08 10:15:21.691000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\1f0e12e1\4Kbx.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\test_basic_isolated.rs',
            'timestamp': '2025-08-08 10:15:21.755000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\27394291\Gbk5.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\test_batch_optimized.rs',
            'timestamp': '2025-08-08 10:15:21.987000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\4b99f42\tC8y.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\test_cache_migration.rs',
            'timestamp': '2025-08-08 10:15:22.227000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-1c22ecc5\Dtus.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\test_cache_service.rs',
            'timestamp': '2025-08-08 10:15:22.410000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\45437041\9lYN.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\test_comprehensive_core.rs',
            'timestamp': '2025-08-08 10:15:22.492000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-77b84344\ipw0.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\test_core_memory_service.rs',
            'timestamp': '2025-08-08 10:15:22.529000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\4072142d\deXV.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\test_critical_unit_tests.rs',
            'timestamp': '2025-08-08 10:15:22.563000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-84fef57\WHV5.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\test_di_async.rs',
            'timestamp': '2025-08-08 10:15:22.598000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-5d5d0c93\eQUy.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\test_di_memory_service_comprehensive.rs',
            'timestamp': '2025-08-08 10:15:22.640000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\322fdd81\8EQR.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\test_di_performance_comparison.rs',
            'timestamp': '2025-08-08 10:15:22.700000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\59d08d2\D9qC.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\test_error_scenarios_comprehensive.rs',
            'timestamp': '2025-08-08 10:15:22.732000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-1788c047\HxWR.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\test_gpu_batch_processor.rs',
            'timestamp': '2025-08-08 10:15:22.771000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-2988fe4b\YZZZ.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\test_hnsw_property_based.rs',
            'timestamp': '2025-08-08 10:15:22.807000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\7c430ac0\odTe.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\test_hnsw_property_based_comprehensive.rs',
            'timestamp': '2025-08-08 10:15:22.848000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\3aadfd84\QHGw.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\test_ml_promotion_decomposed.rs',
            'timestamp': '2025-08-08 10:15:22.889000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-6cc891b4\7gfS.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\test_mocks_and_stubs.rs',
            'timestamp': '2025-08-08 10:15:22.932000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\1eb56fb0\rLQL.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\test_promotion.rs',
            'timestamp': '2025-08-08 10:15:22.980000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\57e89e57\YXGW.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\test_qwen3_complete.rs',
            'timestamp': '2025-08-08 10:15:23.014000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-7de38e30\iDIQ.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\test_service_di_decomposed.rs',
            'timestamp': '2025-08-08 10:15:23.050000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\11353e1a\DKPf.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\test_types.rs',
            'timestamp': '2025-08-08 10:15:23.091000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-1c2062da\rKrh.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\test_unified_api.rs',
            'timestamp': '2025-08-08 10:15:23.126000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-6eadd6ea\u93u.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\test_unified_factory_architecture.rs',
            'timestamp': '2025-08-08 10:15:23.160000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-20e718da\DQCl.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\test_vector_store.rs',
            'timestamp': '2025-08-08 10:15:23.197000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\7279149a\RoPC.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\examples\test_gpu_acceleration.rs',
            'timestamp': '2025-08-08 10:15:23.236000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\45f85599\myd7.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\examples\memory_demo.rs',
            'timestamp': '2025-08-08 10:15:23.274000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-7b9584fd\Xh8b.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\examples\di_best_practices.rs',
            'timestamp': '2025-08-08 10:15:23.308000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-42f01e6f\j2BP.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\examples\comprehensive_performance_validation.rs',
            'timestamp': '2025-08-08 10:15:23.341000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\4b590431\soDN.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\examples\simd_optimized_benchmark.rs',
            'timestamp': '2025-08-08 10:15:23.378000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-5a94173a\GzOj.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\unit\test_unified_container.rs',
            'timestamp': '2025-08-08 10:15:23.478000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\6c12e99d\dq4a.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\unit\test_unified_factory.rs',
            'timestamp': '2025-08-08 10:15:23.584000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-732de02b\yVSP.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\unit\test_di_errors.rs',
            'timestamp': '2025-08-08 10:15:23.640000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\9df8575\OhEi.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\unit\test_unified_config.rs',
            'timestamp': '2025-08-08 10:15:23.676000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-2b0a7ec9\YDKe.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\tests\test_working_unit_tests.rs',
            'timestamp': '2025-08-08 10:15:23.710000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\4339c0c4\QbXr.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\tests\test_model_downloader_comprehensive.rs',
            'timestamp': '2025-08-08 10:15:23.750000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-523594e0\iEj2.toml',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\Cargo.toml',
            'timestamp': '2025-08-08 10:15:23.780000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\6d741540\U48A.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\tests\test_memory_pool.rs',
            'timestamp': '2025-08-08 10:15:23.817000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\3a2792e2\KoEm.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\tests\test_embeddings_gpu_advanced.rs',
            'timestamp': '2025-08-08 10:15:23.856000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\205a4c39\1d1R.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\tests\test_gpu_integration.rs',
            'timestamp': '2025-08-08 10:15:23.892000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-25d92d75\UwTj.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\tests\test_gpu_detector.rs',
            'timestamp': '2025-08-08 10:15:23.925000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-5525eeb1\kvQB.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\tests\test_memory_optimization.rs',
            'timestamp': '2025-08-08 10:15:23.960000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\11d09def\zgvc.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\tests\test_gpu_fallback.rs',
            'timestamp': '2025-08-08 10:15:23.995000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-cb15246\miGj.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\tests\test_gpu_pipeline_comprehensive.rs',
            'timestamp': '2025-08-08 10:15:24.029000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\4be3c60f\xTaS.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\tests\test_gpu_config.rs',
            'timestamp': '2025-08-08 10:15:24.073000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\688a82ce\BOwh.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\src\config.rs',
            'timestamp': '2025-08-08 10:15:24.122000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-565bd877\YMjJ.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\src\lib.rs',
            'timestamp': '2025-08-08 10:15:24.161000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\5c64d36b\wbyn.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\src\reranking.rs',
            'timestamp': '2025-08-08 10:15:24.198000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-76a2d5e1\e7lj.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\src\reranker_qwen3.rs',
            'timestamp': '2025-08-08 10:15:24.238000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-23f89689\qZnq.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\src\reranker_qwen3_optimized.rs',
            'timestamp': '2025-08-08 10:15:24.278000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-53d045fb\ZQT4.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\src\errors.rs',
            'timestamp': '2025-08-08 10:15:24.313000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-367b4c88\KU7n.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\src\auto_device_selector.rs',
            'timestamp': '2025-08-08 10:15:24.348000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\334015be\RuEs.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\tests\test_qwen3_tokenization.rs',
            'timestamp': '2025-08-08 10:15:24.382000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-c7cdb21\UntI.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\tests\test_embeddings_cpu.rs',
            'timestamp': '2025-08-08 10:15:24.412000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\13133071\4fVI.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\tests\test_model_registry.rs',
            'timestamp': '2025-08-08 10:15:24.445000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\658182a3\MGAi.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\tests\test_reranker_integration.rs',
            'timestamp': '2025-08-08 10:15:24.480000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-44b4086d\QZiB.toml',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\tools\Cargo.toml',
            'timestamp': '2025-08-08 10:15:24.515000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-e49b1fa\7GnS.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\src\embeddings_bge_m3.rs',
            'timestamp': '2025-08-08 10:15:24.550000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-6d7d2989\UQ5R.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\ai\src\tokenizer.rs',
            'timestamp': '2025-08-08 10:15:24.591000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-682e31b2\UIBW.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\domain\src\value_objects\access_pattern.rs',
            'timestamp': '2025-08-08 10:15:24.632000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\1df8e051\kai4.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\llm\tests\test_llm_advanced.rs',
            'timestamp': '2025-08-08 10:15:24.674000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\de1a908\4ydn.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\llm\tests\test_llm_client.rs',
            'timestamp': '2025-08-08 10:15:24.718000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-72933c6e\Kktq.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\llm\src\integration_test.rs',
            'timestamp': '2025-08-08 10:15:24.758000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-61627aa9\bZPL.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\llm\tests\test_llm_integration.rs',
            'timestamp': '2025-08-08 10:15:24.798000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-35624b52\iJ77.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\tools\src\file_ops.rs',
            'timestamp': '2025-08-08 10:15:24.829000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\2856d0bc\wlYU.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\tools\src\git_ops.rs',
            'timestamp': '2025-08-08 10:15:24.869000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-519ea7e2\4VLj.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\tools\src\shell_ops.rs',
            'timestamp': '2025-08-08 10:15:24.903000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-31f90106\w9Zy.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\tools\src\web_ops.rs',
            'timestamp': '2025-08-08 10:15:24.937000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-7c032396\ibYA.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\llm\src\lib.rs',
            'timestamp': '2025-08-08 10:15:24.974000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\a83ce14\3qCE.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\commands\memory.rs',
            'timestamp': '2025-08-08 10:15:25.008000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-1067d525\3aQX.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\src\api.rs',
            'timestamp': '2025-08-08 10:15:25.041000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\7d243ae7\gvET.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\llm\src\agents\intent_analyzer.rs',
            'timestamp': '2025-08-08 10:15:25.079000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\2fd669e5\EQBV.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\llm\src\agents\action_planner.rs',
            'timestamp': '2025-08-08 10:15:25.120000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-52cefc5f\6GHL.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\src\gpu_accelerated.rs',
            'timestamp': '2025-08-08 10:15:25.154000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\7d1a4a11\Skyk.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\src\services\refactored_di_memory_service.rs',
            'timestamp': '2025-08-08 10:15:25.194000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-188458c8\7dti.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\llm\src\multi_provider.rs',
            'timestamp': '2025-08-08 10:15:25.229000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\40216ac\1aFd.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\llm\src\cost_optimizer.rs',
            'timestamp': '2025-08-08 10:15:25.264000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-5c6ba04a\i3Eu',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\Makefile',
            'timestamp': '2025-08-08 10:15:25.295000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\4c81e4bd\8VGC.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\commands\gpu.rs',
            'timestamp': '2025-08-08 10:15:25.333000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\7a1ec469\FrS8.toml',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\config\config.example.toml',
            'timestamp': '2025-08-08 10:15:25.370000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\2bcfbc0b\iBcv.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\commands\models.rs',
            'timestamp': '2025-08-08 10:15:25.405000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-1cff928a\UZar.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\tools\src\intelligent_selector.rs',
            'timestamp': '2025-08-08 10:15:25.444000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-2b242f8a\sSQ1.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\tools\src\performance_monitor.rs',
            'timestamp': '2025-08-08 10:15:25.480000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-41312f0c\47hT.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\src\simd_optimized.rs',
            'timestamp': '2025-08-08 10:15:25.515000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\40f0e1a7\rrKo.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\src\simd_ultra_optimized.rs',
            'timestamp': '2025-08-08 10:15:25.549000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\4aa90f48\jUL9.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\router\src\lib.rs',
            'timestamp': '2025-08-08 10:15:25.590000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-522516f5\h5ar.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\domain\src\services\promotion_domain_service.rs',
            'timestamp': '2025-08-08 10:15:25.621000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-eec3245\9WT3.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\domain\src\value_objects\layer_type.rs',
            'timestamp': '2025-08-08 10:15:25.655000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-6ef4db6c\oKu2.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\orchestrator\adaptive_orchestrator.rs',
            'timestamp': '2025-08-08 10:15:25.691000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\7948363d\Odl9.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\llm\src\providers\local_provider.rs',
            'timestamp': '2025-08-08 10:15:25.725000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-52faf8c2\22iK.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\llm\src\providers\openai_provider.rs',
            'timestamp': '2025-08-08 10:15:25.757000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\66df464b\Odwl.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\domain\src\entities\memory_record.rs',
            'timestamp': '2025-08-08 10:15:25.792000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\7d585d23\U3LC.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\tools\src\plugins\hot_reload.rs',
            'timestamp': '2025-08-08 10:15:25.827000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-54492696\vZ2p.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\tools\src\registry\secure_registry.rs',
            'timestamp': '2025-08-08 10:15:25.861000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\2d5dddd9\4Uew.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\tools\src\registry\tool_metadata.rs',
            'timestamp': '2025-08-08 10:15:25.899000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-42411445\ZAPV.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\orchestrator\resource_manager.rs',
            'timestamp': '2025-08-08 10:15:25.933000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\49d7af07\2LXO.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\orchestrator\task_analyzer.rs',
            'timestamp': '2025-08-08 10:15:25.968000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-7bb151e4\bAxg.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\orchestrator\strategy_selector.rs',
            'timestamp': '2025-08-08 10:15:26.002000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-3c64f04a\0Goa.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\orchestrator\tool_orchestrator.rs',
            'timestamp': '2025-08-08 10:15:26.038000',
        },
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-5f24ed47\ScMT.md',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\docs\troubleshooting\Troubleshooting Guide - Common Issues & Solutions.md',
            'timestamp': '2025-08-08 10:15:26.078000',
        },
    ]

    print(f'Restoring {len(changes)} file versions...')
    
    for i, change in enumerate(changes, 1):
        source = Path(change['source'])
        target = Path(change['target'])
        
        # Create directories if needed
        target.parent.mkdir(parents=True, exist_ok=True)
        
        # Copy file
        try:
            shutil.copy2(source, target)
            print(f'[{i}/{len(changes)}] Restored: {target.name} ({change["timestamp"]})')
        except Exception as e:
            print(f'[{i}/{len(changes)}] Failed: {target.name} - {e}')

if __name__ == '__main__':
    restore_changes()