/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package com.vaticle.typedb.core.graph.adjacency.impl;

import com.vaticle.typedb.core.common.collection.KeyValue;
import com.vaticle.typedb.core.common.iterator.FunctionalIterator;
import com.vaticle.typedb.core.common.iterator.sorted.SortedIterator;
import com.vaticle.typedb.core.common.iterator.sorted.SortedIterator.Forwardable;
import com.vaticle.typedb.core.common.parameters.Order;
import com.vaticle.typedb.core.graph.adjacency.TypeAdjacency;
import com.vaticle.typedb.core.encoding.Encoding;
import com.vaticle.typedb.core.graph.edge.TypeEdge;
import com.vaticle.typedb.core.graph.edge.impl.TypeEdgeImpl;
import com.vaticle.typedb.core.graph.vertex.TypeVertex;

import static com.vaticle.typedb.common.collection.Collections.list;
import static com.vaticle.typedb.core.common.parameters.Order.Asc.ASC;
import static com.vaticle.typedb.core.common.iterator.sorted.SortedIterators.iterateSorted;

public abstract class TypeEdgeIterator {

    static class InEdgeIteratorImpl implements TypeAdjacency.In.InEdgeIterator {

        final TypeVertex owner;
        final Forwardable<TypeEdge.View.Backward, Order.Asc> edges;
        final Encoding.Edge.Type encoding;

        InEdgeIteratorImpl(Forwardable<TypeEdge.View.Backward, Order.Asc> edges, TypeVertex owner, Encoding.Edge.Type encoding) {
            this.owner = owner;
            this.edges = edges;
            this.encoding = encoding;
        }

        @Override
        public Forwardable<TypeVertex, Order.Asc> from() {
            return edges.mapSorted(edgeView -> edgeView.edge().from(), this::targetEdge, ASC);
        }

        @Override
        public SortedIterator<TypeVertex, Order.Asc> to() {
            return iterateSorted(ASC, list(owner));
        }

        @Override
        public FunctionalIterator<TypeVertex> overridden() {
            return edges.map(edgeView -> edgeView.edge().overridden().orElse(null)).noNulls();
        }

        @Override
        public Forwardable<KeyValue<TypeVertex, TypeVertex>, Order.Asc> fromAndOverridden() {
            return edges.mapSorted(
                    edgeView -> KeyValue.of(edgeView.edge().from(), edgeView.edge().overridden().orElse(null)),
                    fromAndOverridden -> targetEdge(fromAndOverridden.key()),
                    ASC
            );
        }

        TypeEdge.View.Backward targetEdge(TypeVertex targetFrom) {
            return new TypeEdgeImpl.Target(encoding, targetFrom, owner).backwardView();
        }
    }

    static class OutEdgeIteratorImpl implements TypeAdjacency.Out.OutEdgeIterator {

        final TypeVertex owner;
        final Forwardable<TypeEdge.View.Forward, Order.Asc> edges;
        final Encoding.Edge.Type encoding;

        OutEdgeIteratorImpl(Forwardable<TypeEdge.View.Forward, Order.Asc> edges, TypeVertex owner, Encoding.Edge.Type encoding) {
            this.owner = owner;
            this.edges = edges;
            this.encoding = encoding;
        }

        @Override
        public SortedIterator<TypeVertex, Order.Asc> from() {
            return iterateSorted(ASC, list(owner));
        }

        @Override
        public Forwardable<TypeVertex, Order.Asc> to() {
            return edges.mapSorted(edgeView -> edgeView.edge().to(), this::targetEdge, ASC);
        }

        @Override
        public FunctionalIterator<TypeVertex> overridden() {
            return edges.map(edgeView -> edgeView.edge().overridden().orElse(null)).noNulls();
        }

        @Override
        public Forwardable<KeyValue<TypeVertex, TypeVertex>, Order.Asc> toAndOverridden() {
            return edges.mapSorted(
                    edgeView -> KeyValue.of(edgeView.edge().to(), edgeView.edge().overridden().orElse(null)),
                    toAndOverridden -> targetEdge(toAndOverridden.key()),
                    ASC
            );
        }

        TypeEdge.View.Forward targetEdge(TypeVertex targetTo) {
            return new TypeEdgeImpl.Target(encoding, owner, targetTo).forwardView();
        }
    }
}
