/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package com.vaticle.typedb.core.test.behaviour.reasoner.verification.test;

import com.vaticle.typedb.core.TypeDB;
import com.vaticle.typedb.core.common.parameters.Arguments;
import com.vaticle.typedb.core.common.parameters.Options;
import com.vaticle.typedb.core.database.CoreDatabaseManager;
import com.vaticle.typedb.core.database.CoreSession;
import com.vaticle.typedb.core.database.CoreTransaction;
import com.vaticle.typedb.core.test.behaviour.reasoner.verification.CorrectnessVerifier;
import com.vaticle.typedb.core.test.behaviour.reasoner.verification.CorrectnessVerifier.CompletenessException;
import com.vaticle.typedb.core.test.behaviour.reasoner.verification.CorrectnessVerifier.SoundnessException;
import com.vaticle.typedb.core.test.integration.util.Util;
import com.vaticle.typeql.lang.query.TypeQLGet;
import org.junit.After;
import org.junit.Before;
import org.junit.Test;

import java.io.IOException;
import java.nio.file.Path;
import java.nio.file.Paths;

import static com.vaticle.typedb.common.collection.Collections.list;
import static com.vaticle.typedb.core.common.collection.Bytes.MB;
import static com.vaticle.typedb.core.common.test.Util.assertNotThrows;
import static com.vaticle.typedb.core.common.test.Util.assertThrows;
import static com.vaticle.typeql.lang.TypeQL.and;
import static com.vaticle.typeql.lang.TypeQL.cVar;
import static com.vaticle.typeql.lang.TypeQL.define;
import static com.vaticle.typeql.lang.TypeQL.parseQuery;
import static com.vaticle.typeql.lang.TypeQL.rule;
import static com.vaticle.typeql.lang.TypeQL.type;
import static com.vaticle.typeql.lang.common.TypeQLArg.ValueType.BOOLEAN;
import static com.vaticle.typeql.lang.common.TypeQLToken.Type.ATTRIBUTE;
import static com.vaticle.typeql.lang.common.TypeQLToken.Type.ENTITY;

public class CorrectnessVerifierTest {

    private static final String database = "CorrectnessVerifierTest";
    private static final Path dataDir = Paths.get(System.getProperty("user.dir")).resolve(database);
    private static final Path logDir = dataDir.resolve("logs");
    private static final Options.Database options = new Options.Database().dataDir(dataDir).reasonerDebuggerDir(logDir)
            .storageIndexCacheSize(MB).storageDataCacheSize(MB);
    private CoreDatabaseManager databaseMgr;

    @Before
    public void setUp() throws IOException {
        Util.resetDirectory(dataDir);
        this.databaseMgr = CoreDatabaseManager.open(options);
        this.databaseMgr.create(database);
        try (TypeDB.Session session = databaseMgr.session(CorrectnessVerifierTest.database, Arguments.Session.Type.SCHEMA)) {
            try (TypeDB.Transaction tx = session.transaction(Arguments.Transaction.Type.WRITE)) {
                tx.query().define(define(list(
                        type("employable").sub(ATTRIBUTE).value(BOOLEAN),
                        type("person").sub(ENTITY).owns("employable"),
                        rule("people-are-employable")
                                .when(and(cVar("p").isa("person")))
                                .then(cVar("p").has("employable", true))
                )));
                tx.commit();
            }
        }
        try (TypeDB.Session session = databaseMgr.session(CorrectnessVerifierTest.database, Arguments.Session.Type.DATA)) {
            try (TypeDB.Transaction tx = session.transaction(Arguments.Transaction.Type.WRITE)) {
                tx.query().insert(parseQuery("insert $p isa person;").asInsert());
                tx.commit();
            }
        }
    }

    @After
    public void tearDown() {
        this.databaseMgr.close();
    }

    @Test
    public void testCorrectnessPassesForEmployableExample() {
        TypeQLGet inferenceQuery = parseQuery("match $x has employable true;").asGet();
        try (CoreSession session = databaseMgr.session(database, Arguments.Session.Type.DATA)) {
            CorrectnessVerifier correctnessVerifier = CorrectnessVerifier.initialise(session);
            correctnessVerifier.verifyCorrectness(inferenceQuery);
            correctnessVerifier.close();
        }
    }

    @Test
    public void testSoundnessThrowsWhenRuleTriggersTooOftenEmployableExample() {
        TypeQLGet inferenceQuery = parseQuery("match $x has employable true;").asGet();
        CorrectnessVerifier correctnessVerifier;
        try (CoreSession session = databaseMgr.session(database, Arguments.Session.Type.DATA)) {
            correctnessVerifier = CorrectnessVerifier.initialise(session);
            try (CoreTransaction tx = session.transaction(Arguments.Transaction.Type.WRITE)) {
                tx.query().insert(parseQuery("insert $p isa person;"));
                tx.commit();
            }
            assertThrows(() -> correctnessVerifier.verifySoundness(inferenceQuery), SoundnessException.class);
            assertNotThrows(() -> correctnessVerifier.verifyCompleteness(inferenceQuery));
        }
    }

    @Test
    public void testCompletenessThrowsWhenRuleIsNotTriggeredEmployableExample() {
        TypeQLGet inferenceQuery = parseQuery("match $x has employable true;").asGet();
        CorrectnessVerifier correctnessVerifier;
        try (CoreSession session = databaseMgr.session(database, Arguments.Session.Type.DATA)) {
            correctnessVerifier = CorrectnessVerifier.initialise(session);
            try (CoreTransaction tx = session.transaction(Arguments.Transaction.Type.WRITE)) {
                tx.query().delete(parseQuery("match $p isa person; delete $p isa person;"));
                tx.commit();
            }
            assertThrows(() -> correctnessVerifier.verifyCompleteness(inferenceQuery), CompletenessException.class);
            assertNotThrows(() -> correctnessVerifier.verifySoundness(inferenceQuery));
        }
    }

}